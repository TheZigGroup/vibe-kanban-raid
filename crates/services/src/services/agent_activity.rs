//! Service for autonomous task selection using AI analysis.

use std::{path::Path, sync::Arc, time::Duration};

use async_trait::async_trait;
use db::{
    DBService,
    models::{
        agent_activity::{
            AgentAction, AgentActivityLog, AgentActivityStatus, AgentTriggerResponse,
            ProjectAgentSettings,
        },
        project_repo::ProjectRepo,
        task::{Task, TaskStatus, TaskWithAttemptStatus},
        workspace::{CreateWorkspace, Workspace},
        workspace_repo::{CreateWorkspaceRepo, WorkspaceRepo},
    },
};
use executors::profile::ExecutorProfileId;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::{sync::RwLock, time::interval};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{
    claude_api::{ClaudeApiClient, ClaudeApiError},
    config::Config,
    git::GitService,
    notification::NotificationService,
};

#[derive(Debug, Error)]
pub enum AgentActivityError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("claude api error: {0}")]
    ClaudeApi(#[from] ClaudeApiError),
    #[error("no tasks available")]
    NoTasksAvailable,
    #[error("task already in progress")]
    TaskAlreadyInProgress,
    #[error("agent activity not enabled")]
    NotEnabled,
    #[error("workspace creation failed: {0}")]
    WorkspaceCreation(String),
    #[error("no repositories for project")]
    NoRepositories,
}

/// Trait for starting workspaces - implemented by container services
#[async_trait]
pub trait WorkspaceStarter: Send + Sync {
    /// Generate a git branch name for a workspace
    async fn git_branch_from_workspace(&self, workspace_id: &Uuid, task_title: &str) -> String;

    /// Start a workspace with the given executor profile
    async fn start_workspace(
        &self,
        workspace: &Workspace,
        executor_profile_id: ExecutorProfileId,
    ) -> Result<(), String>;
}

/// Response from AI task selection
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskSelectionResponse {
    task_id: String,
    reasoning: String,
}

/// Task info sent to AI for selection
#[derive(Debug, Clone, Serialize)]
struct TaskInfo {
    id: String,
    title: String,
    description: Option<String>,
    layer: Option<String>,
    sequence: Option<i32>,
}

/// Configuration for auto-attempt feature
pub struct AutoAttemptConfig {
    pub git_service: GitService,
    pub config: Arc<RwLock<Config>>,
    pub workspace_starter: Arc<dyn WorkspaceStarter>,
}

/// Background service for autonomous task selection
pub struct AgentActivityService {
    db: DBService,
    notification_service: NotificationService,
    poll_interval: Duration,
    auto_attempt: Option<AutoAttemptConfig>,
}

impl AgentActivityService {
    /// Spawn the background agent activity service
    pub async fn spawn(
        db: DBService,
        notification_service: NotificationService,
        auto_attempt: Option<AutoAttemptConfig>,
    ) -> tokio::task::JoinHandle<()> {
        let service = Self {
            db,
            notification_service,
            poll_interval: Duration::from_secs(60), // Check every minute
            auto_attempt,
        };
        tokio::spawn(async move {
            service.start().await;
        })
    }

    async fn start(&self) {
        info!(
            "Starting agent activity service with interval {:?}",
            self.poll_interval
        );

        // Check if ANTHROPIC_API_KEY is set at startup
        if std::env::var("ANTHROPIC_API_KEY").is_err() {
            warn!("ANTHROPIC_API_KEY not set - agent activity AI task selection will fail");
        }

        let mut interval = interval(self.poll_interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.check_all_enabled_projects().await {
                error!("Error checking enabled projects for agent activity: {}", e);
            }
        }
    }

    /// Check all projects with agent activity enabled
    async fn check_all_enabled_projects(&self) -> Result<(), AgentActivityError> {
        let enabled_projects = ProjectAgentSettings::find_all_enabled(&self.db.pool).await?;

        if enabled_projects.is_empty() {
            debug!("Agent activity: no projects with agent activity enabled");
            return Ok(());
        }

        info!(
            "Agent activity: checking {} projects with agent activity enabled",
            enabled_projects.len()
        );

        for settings in enabled_projects {
            match Self::check_and_select_next_task(
                &self.db.pool,
                &self.notification_service,
                settings.project_id,
                self.auto_attempt.as_ref(),
            )
            .await
            {
                Ok(response) => {
                    if response.action == AgentAction::Selected {
                        info!(
                            project_id = %settings.project_id,
                            action = %response.action,
                            task_id = ?response.task_id,
                            "Agent activity: task selected"
                        );
                    }
                }
                Err(AgentActivityError::TaskAlreadyInProgress) => {
                    // Normal case, skip silently
                    debug!(
                        project_id = %settings.project_id,
                        "Agent activity: task already in progress, skipping"
                    );
                }
                Err(AgentActivityError::NoTasksAvailable) => {
                    // Normal case, skip silently
                    debug!(
                        project_id = %settings.project_id,
                        "Agent activity: no tasks available"
                    );
                }
                Err(e) => {
                    warn!(
                        project_id = %settings.project_id,
                        error = %e,
                        "Agent activity cycle failed"
                    );
                }
            }
        }

        Ok(())
    }
}

impl AgentActivityService {
    /// Main entry point: check conditions and select next task if applicable
    pub async fn check_and_select_next_task(
        pool: &SqlitePool,
        notification_service: &NotificationService,
        project_id: Uuid,
        auto_attempt: Option<&AutoAttemptConfig>,
    ) -> Result<AgentTriggerResponse, AgentActivityError> {
        // Get all tasks for the project to check status
        let all_tasks = Task::find_by_project_id_with_attempt_status(pool, project_id).await?;

        // Check if any task is in progress or in review status
        // Skip auto-selection when work is actively being done or reviewed
        let has_active_task = all_tasks
            .iter()
            .any(|t| t.status == TaskStatus::InProgress || t.status == TaskStatus::InReview);

        if has_active_task {
            return Err(AgentActivityError::TaskAlreadyInProgress);
        }

        // Filter to only todo tasks and convert to Task for AI selection
        let tasks: Vec<TaskWithAttemptStatus> = all_tasks
            .into_iter()
            .filter(|t| t.status == TaskStatus::Todo)
            .collect();

        if tasks.is_empty() {
            // Log the skip
            AgentActivityLog::create(
                pool,
                project_id,
                None,
                AgentAction::Skipped,
                Some("No todo tasks available".to_string()),
            )
            .await?;

            return Ok(AgentTriggerResponse {
                action: AgentAction::Skipped,
                task_id: None,
                reasoning: Some("No todo tasks available".to_string()),
            });
        }

        info!(
            project_id = %project_id,
            todo_count = tasks.len(),
            "Agent activity: found todo tasks, using AI to select next task"
        );

        // Use AI to select the best task
        match Self::select_task_with_ai(&tasks).await {
            Ok((task_id, reasoning)) => {
                // Update task status to in progress
                Task::update_status(pool, task_id, TaskStatus::InProgress).await?;

                // Log the selection
                AgentActivityLog::create(
                    pool,
                    project_id,
                    Some(task_id),
                    AgentAction::Selected,
                    Some(reasoning.clone()),
                )
                .await?;

                // Get the task for notifications and auto-attempt
                let task = Task::find_by_id(pool, task_id).await?;

                // Send notification
                if let Some(ref task) = task {
                    notification_service
                        .notify("Task Selected", &format!("Starting: {}", task.title))
                        .await;
                }

                // Auto-attempt: create and start workspace if configured
                if let (Some(auto_attempt_config), Some(task)) = (auto_attempt, task) {
                    if let Err(e) = Self::auto_start_attempt(
                        pool,
                        &task,
                        project_id,
                        auto_attempt_config,
                    )
                    .await
                    {
                        warn!(
                            task_id = %task_id,
                            error = %e,
                            "Failed to auto-start attempt for task"
                        );
                        // Don't fail the whole operation, just log
                    } else {
                        info!(task_id = %task_id, "Auto-started attempt for selected task");
                    }
                }

                Ok(AgentTriggerResponse {
                    action: AgentAction::Selected,
                    task_id: Some(task_id),
                    reasoning: Some(reasoning),
                })
            }
            Err(e) => {
                // Log the error
                AgentActivityLog::create(
                    pool,
                    project_id,
                    None,
                    AgentAction::Error,
                    Some(e.to_string()),
                )
                .await?;

                Err(e)
            }
        }
    }

    /// Auto-start an attempt for a task using default settings
    async fn auto_start_attempt(
        pool: &SqlitePool,
        task: &Task,
        project_id: Uuid,
        auto_attempt: &AutoAttemptConfig,
    ) -> Result<(), AgentActivityError> {
        // Get repos for the project
        let repos = ProjectRepo::find_repos_for_project(pool, project_id).await?;

        if repos.is_empty() {
            return Err(AgentActivityError::NoRepositories);
        }

        // Get executor profile from config
        let executor_profile_id = auto_attempt.config.read().await.executor_profile.clone();

        // Generate workspace ID and branch name
        let workspace_id = Uuid::new_v4();
        let git_branch_name = auto_attempt
            .workspace_starter
            .git_branch_from_workspace(&workspace_id, &task.title)
            .await;

        // Compute agent_working_dir based on repo count:
        // - Single repo: use repo name as working dir
        // - Multiple repos: use None (agent runs in workspace root)
        let agent_working_dir = if repos.len() == 1 {
            Some(repos[0].name.clone())
        } else {
            None
        };

        // Create workspace
        let workspace = Workspace::create(
            pool,
            &CreateWorkspace {
                branch: git_branch_name,
                agent_working_dir,
            },
            workspace_id,
            task.id,
        )
        .await
        .map_err(|e| AgentActivityError::WorkspaceCreation(e.to_string()))?;

        // Create workspace repos with target branches (current branch of each repo)
        let workspace_repos: Vec<CreateWorkspaceRepo> = repos
            .iter()
            .map(|repo| {
                // Get current branch for the repo, fallback to "main"
                let target_branch = auto_attempt
                    .git_service
                    .get_current_branch(Path::new(&repo.path))
                    .unwrap_or_else(|_| "main".to_string());

                CreateWorkspaceRepo {
                    repo_id: repo.id,
                    target_branch,
                }
            })
            .collect();

        WorkspaceRepo::create_many(pool, workspace.id, &workspace_repos).await?;

        // Start the workspace
        auto_attempt
            .workspace_starter
            .start_workspace(&workspace, executor_profile_id)
            .await
            .map_err(AgentActivityError::WorkspaceCreation)?;

        info!(
            workspace_id = %workspace.id,
            task_id = %task.id,
            "Auto-started workspace for task"
        );

        Ok(())
    }

    /// Use AI to select the best task from the list
    async fn select_task_with_ai(
        tasks: &[TaskWithAttemptStatus],
    ) -> Result<(Uuid, String), AgentActivityError> {
        let claude = ClaudeApiClient::from_env()?;

        // Convert tasks to simplified format for AI
        let task_infos: Vec<TaskInfo> = tasks
            .iter()
            .map(|t| TaskInfo {
                id: t.id.to_string(),
                title: t.title.clone(),
                description: t.description.clone(),
                layer: t.layer.as_ref().map(|l| l.to_string()),
                sequence: t.sequence,
            })
            .collect();

        let tasks_json = serde_json::to_string_pretty(&task_infos)
            .map_err(|e| AgentActivityError::ClaudeApi(ClaudeApiError::Serde(e.to_string())))?;

        let prompt = format!(
            r#"You are a task prioritization assistant. Analyze the following tasks and select the ONE task that should be worked on next.

## Prioritization Rules:
1. Respect sequence order (lower sequence = higher priority)
2. Respect layer dependencies: data → backend → frontend → fullstack → devops → testing
3. Consider task descriptions for urgency indicators
4. Prefer tasks that unblock other work

## Tasks:
{tasks_json}

## Output Format:
Return ONLY valid JSON:
{{
  "task_id": "uuid-of-selected-task",
  "reasoning": "Brief explanation of why this task was selected"
}}"#
        );

        let system = Some(
            "You are a task prioritization assistant. Select the most appropriate task to work on next based on dependencies and priority. Output valid JSON only.".to_string(),
        );

        let response: TaskSelectionResponse = claude.ask_json(&prompt, system).await?;

        // Parse and validate the task ID
        let task_id = Uuid::parse_str(&response.task_id).map_err(|_| {
            AgentActivityError::ClaudeApi(ClaudeApiError::Serde(format!(
                "Invalid task ID from AI: {}",
                response.task_id
            )))
        })?;

        // Verify the task exists and belongs to this project
        if !tasks.iter().any(|t| t.id == task_id) {
            return Err(AgentActivityError::ClaudeApi(ClaudeApiError::Serde(
                format!("AI selected invalid task ID: {}", task_id),
            )));
        }

        Ok((task_id, response.reasoning))
    }

    /// Get the current agent activity status for a project
    pub async fn get_status(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<AgentActivityStatus, AgentActivityError> {
        let settings = ProjectAgentSettings::find_by_project_id(pool, project_id).await?;
        let latest_log = AgentActivityLog::find_latest_by_project_id(pool, project_id).await?;

        Ok(AgentActivityStatus {
            enabled: settings.as_ref().map(|s| s.enabled).unwrap_or(false),
            interval_seconds: settings.as_ref().map(|s| s.interval_seconds).unwrap_or(60),
            last_run: latest_log.as_ref().map(|l| l.created_at),
            last_selected_task_id: latest_log
                .as_ref()
                .filter(|l| l.action == AgentAction::Selected)
                .and_then(|l| l.task_id),
            last_reasoning: latest_log.and_then(|l| l.reasoning),
        })
    }

    /// Enable agent activity for a project
    pub async fn enable(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<ProjectAgentSettings, AgentActivityError> {
        Ok(ProjectAgentSettings::set_enabled(pool, project_id, true).await?)
    }

    /// Disable agent activity for a project
    pub async fn disable(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<ProjectAgentSettings, AgentActivityError> {
        Ok(ProjectAgentSettings::set_enabled(pool, project_id, false).await?)
    }
}
