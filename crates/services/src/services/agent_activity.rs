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
        task::{CreateTask, Task, TaskLayer, TaskStatus, TaskType, TaskWithAttemptStatus},
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
    task_type: Option<String>,
    sequence: Option<i32>,
}

/// Response from AI complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ComplexityAnalysisResponse {
    complexity_score: i32,
    can_be_broken_down: bool,
    reasoning: String,
    subtasks: Option<Vec<SubtaskSuggestion>>,
}

/// Suggested subtask from AI complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubtaskSuggestion {
    title: String,
    description: String,
    layer: Option<String>,
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
            poll_interval: Duration::from_secs(10), // Check every 10 seconds for faster response
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
    /// Get layers that already have running non-Integration tasks
    /// (layers with InProgress or InReview tasks that are NOT Integration type)
    fn get_active_layers(tasks: &[TaskWithAttemptStatus]) -> Vec<TaskLayer> {
        tasks
            .iter()
            .filter(|t| {
                t.task_type != Some(TaskType::Integration)
                    && (t.status == TaskStatus::InProgress || t.status == TaskStatus::InReview)
            })
            .filter_map(|t| t.layer.clone())
            .collect()
    }

    /// Check if there's an active Integration task
    fn has_active_integration_task(tasks: &[TaskWithAttemptStatus]) -> bool {
        tasks.iter().any(|t| {
            t.task_type == Some(TaskType::Integration)
                && (t.status == TaskStatus::InProgress || t.status == TaskStatus::InReview)
        })
    }

    /// Main entry point: check conditions and select next task if applicable
    pub async fn check_and_select_next_task(
        pool: &SqlitePool,
        notification_service: &NotificationService,
        project_id: Uuid,
        auto_attempt: Option<&AutoAttemptConfig>,
    ) -> Result<AgentTriggerResponse, AgentActivityError> {
        // Get all tasks for the project to check status
        let all_tasks = Task::find_by_project_id_with_attempt_status(pool, project_id).await?;

        // First, check for any Fullstack tasks that need to be broken down
        for task in all_tasks.iter() {
            if task.status == TaskStatus::Todo && task.layer == Some(TaskLayer::Fullstack) {
                if let Some(task_full) = Task::find_by_id(pool, task.id).await? {
                    info!(
                        task_id = %task.id,
                        "Agent activity: breaking down Fullstack task into layers"
                    );
                    if let Ok(created_count) =
                        Self::breakdown_fullstack_task(pool, &task_full, project_id).await
                    {
                        if created_count > 0 {
                            AgentActivityLog::create(
                                pool,
                                project_id,
                                Some(task.id),
                                AgentAction::Replaced,
                                Some(format!(
                                    "Fullstack task broken into {} layer-specific subtasks",
                                    created_count
                                )),
                            )
                            .await?;

                            notification_service
                                .notify(
                                    "Task Breakdown",
                                    &format!(
                                        "Fullstack task '{}' split into {} subtasks",
                                        task.title, created_count
                                    ),
                                )
                                .await;

                            return Ok(AgentTriggerResponse {
                                action: AgentAction::Replaced,
                                task_id: Some(task.id),
                                reasoning: Some(format!(
                                    "Fullstack task broken into {} layer-specific subtasks",
                                    created_count
                                )),
                            });
                        }
                    }
                }
            }
        }

        // Get active layers (layers with InProgress/InReview non-Integration tasks)
        let active_layers = Self::get_active_layers(&all_tasks);
        let active_layer_count = active_layers.len();
        let has_active_integration = Self::has_active_integration_task(&all_tasks);

        // Concurrency rules:
        // 1. Non-Integration tasks can run concurrently by layer (up to 3: Frontend, Backend, Data)
        // 2. Integration tasks run sequentially (only when nothing else is in progress)
        // 3. Mock tasks take priority over Implementation tasks
        // 4. If an Integration task is active, block everything else

        // If there's an active Integration task, block all new tasks
        if has_active_integration {
            return Err(AgentActivityError::TaskAlreadyInProgress);
        }

        // Check for available non-Integration tasks that can run (in a layer not already active)
        let has_available_layered_task = all_tasks.iter().any(|t| {
            t.status == TaskStatus::Todo
                && t.task_type != Some(TaskType::Integration)
                && t.layer
                    .as_ref()
                    .map(|l| !active_layers.contains(l))
                    .unwrap_or(false) // Must have a layer for concurrent execution
        });

        // Check if there's any active task
        let has_any_active_task = all_tasks
            .iter()
            .any(|t| t.status == TaskStatus::InProgress || t.status == TaskStatus::InReview);

        let tasks: Vec<TaskWithAttemptStatus> = if has_available_layered_task && active_layer_count < 3 {
            // Can start a non-Integration task in an available layer
            info!(
                project_id = %project_id,
                active_layer_count = active_layer_count,
                "Agent activity: selecting layered task (concurrent by layer allowed)"
            );

            // Get all eligible non-Integration tasks in available layers
            let eligible: Vec<TaskWithAttemptStatus> = all_tasks
                .clone()
                .into_iter()
                .filter(|t| {
                    t.status == TaskStatus::Todo
                        && t.task_type != Some(TaskType::Integration)
                        && t.layer
                            .as_ref()
                            .map(|l| !active_layers.contains(l))
                            .unwrap_or(false)
                })
                .collect();

            // Prioritize: Architecture > Implementation
            let has_arch = eligible.iter().any(|t| t.task_type == Some(TaskType::Architecture));

            if has_arch {
                eligible.into_iter().filter(|t| t.task_type == Some(TaskType::Architecture)).collect()
            } else {
                eligible
            }
        } else if has_any_active_task {
            // Something is active and we can't start more layered tasks - block
            return Err(AgentActivityError::TaskAlreadyInProgress);
        } else {
            // Nothing active - can start any todo task
            // Priority: Sequence 1 (init) > Architecture > Mock > Implementation > Integration
            let todo_tasks: Vec<TaskWithAttemptStatus> = all_tasks
                .into_iter()
                .filter(|t| t.status == TaskStatus::Todo)
                .collect();

            // CRITICAL: Initialization tasks (sequence=1) have highest priority
            // These set up the project to be runnable
            let has_init = todo_tasks.iter().any(|t| t.sequence == Some(1));
            let has_arch = todo_tasks.iter().any(|t| t.task_type == Some(TaskType::Architecture));
            let has_impl = todo_tasks.iter().any(|t| t.task_type == Some(TaskType::Implementation));

            if has_init {
                // Sequence 1 tasks are initialization - do these first!
                todo_tasks.into_iter().filter(|t| t.sequence == Some(1)).collect()
            } else if has_arch {
                // Architecture tasks set up structure
                todo_tasks.into_iter().filter(|t| t.task_type == Some(TaskType::Architecture)).collect()
            } else if has_impl {
                todo_tasks.into_iter().filter(|t| t.task_type == Some(TaskType::Implementation)).collect()
            } else {
                // Only Integration tasks left
                todo_tasks
            }
        };

        if tasks.is_empty() {
            AgentActivityLog::create(
                pool,
                project_id,
                None,
                AgentAction::Skipped,
                Some("No eligible tasks available".to_string()),
            )
            .await?;

            return Ok(AgentTriggerResponse {
                action: AgentAction::Skipped,
                task_id: None,
                reasoning: Some("No eligible tasks available".to_string()),
            });
        }

        info!(
            project_id = %project_id,
            todo_count = tasks.len(),
            "Agent activity: found eligible tasks, using AI to select next task"
        );

        // Use AI to select the best task
        match Self::select_task_with_ai(&tasks).await {
            Ok((task_id, reasoning)) => {
                let task = Task::find_by_id(pool, task_id)
                    .await?
                    .ok_or(AgentActivityError::NoTasksAvailable)?;

                // Check complexity (skip for subtasks and tasks with prevent_breakdown flag)
                if task.complexity_score.is_none()
                    && task.parent_task_id.is_none()
                    && !task.prevent_breakdown
                {
                    match Self::analyze_complexity_and_maybe_breakdown(
                        pool,
                        &task,
                        project_id,
                        notification_service,
                    )
                    .await
                    {
                        Ok(Some(subtask_count)) => {
                            return Ok(AgentTriggerResponse {
                                action: AgentAction::Replaced,
                                task_id: Some(task_id),
                                reasoning: Some(format!(
                                    "Complex task broken into {} subtasks",
                                    subtask_count
                                )),
                            });
                        }
                        Ok(None) => {}
                        Err(e) => {
                            warn!(
                                task_id = %task_id,
                                error = %e,
                                "Complexity analysis failed, proceeding with task anyway"
                            );
                        }
                    }
                }

                Task::update_status(pool, task_id, TaskStatus::InProgress).await?;

                AgentActivityLog::create(
                    pool,
                    project_id,
                    Some(task_id),
                    AgentAction::Selected,
                    Some(reasoning.clone()),
                )
                .await?;

                notification_service
                    .notify("Task Selected", &format!("Starting: {}", task.title))
                    .await;

                if let Some(auto_attempt_config) = auto_attempt {
                    if let Err(e) =
                        Self::auto_start_attempt(pool, &task, project_id, auto_attempt_config).await
                    {
                        warn!(
                            task_id = %task_id,
                            error = %e,
                            "Failed to auto-start attempt for task"
                        );
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

    /// Break down a Fullstack task into Frontend, Backend, and Data subtasks
    async fn breakdown_fullstack_task(
        pool: &SqlitePool,
        task: &Task,
        _project_id: Uuid,
    ) -> Result<usize, AgentActivityError> {
        let layers = [TaskLayer::Data, TaskLayer::Backend, TaskLayer::Frontend];
        let layer_names = ["Data", "Backend", "Frontend"];
        let mut created_count = 0;

        for (layer, name) in layers.iter().zip(layer_names.iter()) {
            let subtask_title = format!("{} - {} Layer", task.title, name);
            let subtask_description = task.description.clone().map(|d| {
                format!(
                    "{}\n\n[Auto-generated {} layer subtask from Fullstack task]",
                    d, name
                )
            });

            let create_data = CreateTask::subtask_of(
                task.project_id,
                subtask_title,
                subtask_description,
                Some(layer.clone()),
                task.task_type.clone(),
                task.sequence.unwrap_or(0),
                task.testing_criteria.clone(),
                None,
                task.id,
            );

            Task::create(pool, &create_data, Uuid::new_v4()).await?;
            created_count += 1;
        }

        // Cancel the original Fullstack task
        Task::update_status(pool, task.id, TaskStatus::Cancelled).await?;

        Ok(created_count)
    }

    /// Analyze task complexity using AI and break down if needed
    /// Returns Some(count) if task was broken down, None otherwise
    async fn analyze_complexity_and_maybe_breakdown(
        pool: &SqlitePool,
        task: &Task,
        project_id: Uuid,
        notification_service: &NotificationService,
    ) -> Result<Option<usize>, AgentActivityError> {
        let claude = ClaudeApiClient::from_env()?;

        let prompt = format!(
            r#"Analyze the complexity of this software development task:

## Task
Title: {}
Description: {}
Layer: {}
Type: {}

## Criteria for High Complexity (score >= 7):
- Would take > 4 hours of work
- Touches > 3 files/components
- Has unclear boundaries
- Can be split into independently testable parts
- Requires multiple distinct implementation steps

## Output Format (JSON only):
{{
  "complexity_score": <1-10>,
  "can_be_broken_down": <true/false>,
  "reasoning": "<brief explanation>",
  "subtasks": [
    {{"title": "<subtask title>", "description": "<what to do>", "layer": "<data|backend|frontend|null>"}},
    ...
  ]
}}

If complexity_score < 7 or can_be_broken_down is false, subtasks can be empty array.
Limit to 2-4 subtasks maximum if breaking down."#,
            task.title,
            task.description.as_deref().unwrap_or("(no description)"),
            task.layer
                .as_ref()
                .map(|l| l.to_string())
                .unwrap_or_else(|| "unspecified".to_string()),
            task.task_type
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "implementation".to_string()),
        );

        let system = Some(
            "You are a software project complexity analyzer. Analyze tasks and suggest breakdowns for complex work. Output valid JSON only.".to_string()
        );

        let analysis: ComplexityAnalysisResponse = claude.ask_json(&prompt, system).await?;

        // Store complexity score
        Task::update_complexity_score(pool, task.id, analysis.complexity_score).await?;

        info!(
            task_id = %task.id,
            complexity_score = analysis.complexity_score,
            can_breakdown = analysis.can_be_broken_down,
            "Agent activity: complexity analysis complete"
        );

        // Check if we should break down
        if analysis.complexity_score >= 7
            && analysis.can_be_broken_down
            && analysis.subtasks.as_ref().map(|s| s.len()).unwrap_or(0) >= 2
        {
            let subtasks = analysis.subtasks.unwrap();
            let mut created_count = 0;

            for (i, subtask) in subtasks.iter().enumerate() {
                let layer = subtask.layer.as_ref().and_then(|l| match l.as_str() {
                    "data" => Some(TaskLayer::Data),
                    "backend" => Some(TaskLayer::Backend),
                    "frontend" => Some(TaskLayer::Frontend),
                    "fullstack" => Some(TaskLayer::Fullstack),
                    "devops" => Some(TaskLayer::Devops),
                    "testing" => Some(TaskLayer::Testing),
                    _ => task.layer.clone(),
                });

                let create_data = CreateTask::subtask_of(
                    task.project_id,
                    subtask.title.clone(),
                    Some(subtask.description.clone()),
                    layer.or_else(|| task.layer.clone()),
                    task.task_type.clone(),
                    task.sequence.unwrap_or(0) * 10 + i as i32,
                    task.testing_criteria.clone(),
                    None,
                    task.id,
                );

                Task::create(pool, &create_data, Uuid::new_v4()).await?;
                created_count += 1;
            }

            // Cancel the original task
            Task::update_status(pool, task.id, TaskStatus::Cancelled).await?;

            // Log the replacement
            AgentActivityLog::create(
                pool,
                project_id,
                Some(task.id),
                AgentAction::Replaced,
                Some(format!(
                    "Complex task (score {}) broken into {} subtasks: {}",
                    analysis.complexity_score, created_count, analysis.reasoning
                )),
            )
            .await?;

            notification_service
                .notify(
                    "Task Breakdown",
                    &format!(
                        "Complex task '{}' split into {} subtasks",
                        task.title, created_count
                    ),
                )
                .await;

            return Ok(Some(created_count));
        }

        Ok(None)
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
                task_type: t.task_type.as_ref().map(|tt| tt.to_string()),
                sequence: t.sequence,
            })
            .collect();

        let tasks_json = serde_json::to_string_pretty(&task_infos)
            .map_err(|e| AgentActivityError::ClaudeApi(ClaudeApiError::Serde(e.to_string())))?;

        let prompt = format!(
            r#"You are a task prioritization assistant. Analyze the following tasks and select the ONE task that should be worked on next.

## CRITICAL: Prioritization Rules (in strict order):
1. **INITIALIZATION FIRST**: Tasks that initialize or set up the project MUST come first. Look for:
   - Tasks with sequence=1 (highest priority)
   - Architecture tasks that set up project structure, configs, or scaffolding
   - Tasks with titles containing: "init", "setup", "scaffold", "configure", "create project", "initialize"
   - The project must be runnable in the browser after these tasks complete!

2. **Sequence order**: Lower sequence number = higher priority (sequence 1 before 2, 2 before 3, etc.)

3. **Task type order**: architecture → mock → implementation → integration
   - Architecture tasks set up structure (do these early)
   - Mock tasks enable parallel development
   - Implementation tasks build features
   - Integration tasks come last (they wire everything together)

4. **Layer dependencies**: data → backend → frontend → fullstack
   - Data layer should be set up before backend
   - Backend before frontend (frontend needs API endpoints)

5. **Unblocking**: Prefer tasks that enable other tasks to proceed

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
            "You are a task prioritization assistant. Your PRIMARY goal is ensuring the codebase is always runnable. Initialization and setup tasks MUST be completed first. Select the most appropriate task based on strict priority order. Output valid JSON only.".to_string(),
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
