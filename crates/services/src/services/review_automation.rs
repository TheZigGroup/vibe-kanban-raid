//! Service for automated review processing: running tests and auto-merging branches.

use std::{path::Path, process::Stdio, time::Duration};

use db::{
    DBService,
    models::{
        merge::Merge,
        review_automation::{
            ProjectReviewSettings, ReviewAction, ReviewAutomationLog, ReviewAutomationStatus,
        },
        task::{CreateTask, Task, TaskLayer, TaskStatus},
        workspace::Workspace,
        workspace_repo::WorkspaceRepo,
    },
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::{process::Command, time::interval};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{git::GitService, notification::NotificationService};
use super::claude_api::{ClaudeApiClient, ClaudeApiError};

/// Maximum number of merge conflict attempts before cancelling and breaking down the task
const MAX_MERGE_CONFLICT_ATTEMPTS: i64 = 5;

/// Response from AI for breaking down a conflicting task
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConflictBreakdownResponse {
    subtasks: Vec<SubtaskSuggestion>,
    reasoning: String,
}

/// Suggested subtask from AI breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubtaskSuggestion {
    title: String,
    description: String,
    layer: Option<String>,
}

#[derive(Debug, Error)]
pub enum ReviewAutomationError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("git error: {0}")]
    Git(#[from] super::git::GitServiceError),
    #[error("test failed: {0}")]
    TestFailed(String),
    #[error("merge conflict: {0}")]
    MergeConflict(String),
    #[error("no workspace container")]
    NoWorkspaceContainer,
    #[error("command execution failed: {0}")]
    CommandFailed(String),
}

/// Detected project stack for running tests
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectStack {
    NodeJs,
    Rust,
    Python,
    Go,
    Unknown,
}

impl ProjectStack {
    /// Get the test command for this stack
    pub fn test_command(&self) -> Option<(&str, &[&str])> {
        match self {
            ProjectStack::NodeJs => Some(("npm", &["test"])),
            ProjectStack::Rust => Some(("cargo", &["test"])),
            ProjectStack::Python => Some(("pytest", &[])),
            ProjectStack::Go => Some(("go", &["test", "./..."])),
            ProjectStack::Unknown => None,
        }
    }
}

/// Background service for automated review processing
pub struct ReviewAutomationService {
    db: DBService,
    git_service: GitService,
    notification_service: NotificationService,
    poll_interval: Duration,
}

impl ReviewAutomationService {
    /// Spawn the background review automation service
    pub async fn spawn(
        db: DBService,
        git_service: GitService,
        notification_service: NotificationService,
    ) -> tokio::task::JoinHandle<()> {
        let service = Self {
            db,
            git_service,
            notification_service,
            poll_interval: Duration::from_secs(10), // Check every 10 seconds for faster response
        };
        tokio::spawn(async move {
            service.start().await;
        })
    }

    async fn start(&self) {
        info!(
            "Starting review automation service with interval {:?}",
            self.poll_interval
        );

        let mut interval = interval(self.poll_interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.check_all_enabled_projects().await {
                error!("Error checking enabled projects for review automation: {}", e);
            }
        }
    }

    /// Check all projects with review automation enabled
    async fn check_all_enabled_projects(&self) -> Result<(), ReviewAutomationError> {
        let enabled_projects = ProjectReviewSettings::find_all_enabled(&self.db.pool).await?;

        if enabled_projects.is_empty() {
            debug!("Review automation: no projects with review automation enabled");
            return Ok(());
        }

        info!(
            "Review automation: checking {} projects with review automation enabled",
            enabled_projects.len()
        );

        for settings in enabled_projects {
            match self.process_project(&settings).await {
                Ok(Some((task, action))) => {
                    info!(
                        project_id = %settings.project_id,
                        task_id = %task.id,
                        action = %action,
                        "Review automation: processed task"
                    );
                }
                Ok(None) => {
                    debug!(
                        project_id = %settings.project_id,
                        "Review automation: no tasks to process"
                    );
                }
                Err(e) => {
                    warn!(
                        project_id = %settings.project_id,
                        error = %e,
                        "Review automation: error processing project"
                    );
                }
            }
        }

        Ok(())
    }

    /// Process a single project - find and process in-review tasks
    async fn process_project(
        &self,
        settings: &ProjectReviewSettings,
    ) -> Result<Option<(Task, ReviewAction)>, ReviewAutomationError> {
        // Find tasks in review with completed attempts
        let tasks_with_workspaces =
            Task::find_in_review_with_completed_attempts(&self.db.pool, settings.project_id)
                .await?;

        if tasks_with_workspaces.is_empty() {
            return Ok(None);
        }

        // Process the first eligible task
        let (task, workspace) = tasks_with_workspaces.into_iter().next().unwrap();

        let action = self
            .process_task_review(&task, &workspace, settings)
            .await?;

        Ok(Some((task, action)))
    }

    /// Process a single task's review
    async fn process_task_review(
        &self,
        task: &Task,
        workspace: &Workspace,
        settings: &ProjectReviewSettings,
    ) -> Result<ReviewAction, ReviewAutomationError> {
        let workspace_path = workspace.container_ref.as_ref().ok_or_else(|| {
            warn!(
                task_id = %task.id,
                workspace_id = %workspace.id,
                "Review automation: workspace has no container_ref"
            );
            ReviewAutomationError::NoWorkspaceContainer
        })?;

        // Step 1: Run tests if enabled and testing_criteria exists
        if settings.run_tests_enabled && task.testing_criteria.is_some() {
            match self.run_tests(workspace, workspace_path).await {
                Ok(output) => {
                    ReviewAutomationLog::create(
                        &self.db.pool,
                        task.id,
                        workspace.id,
                        ReviewAction::TestPassed,
                        Some(output),
                        None,
                    )
                    .await?;
                }
                Err(ReviewAutomationError::TestFailed(output)) => {
                    ReviewAutomationLog::create(
                        &self.db.pool,
                        task.id,
                        workspace.id,
                        ReviewAction::TestFailed,
                        Some(output.clone()),
                        Some("Tests failed".to_string()),
                    )
                    .await?;

                    self.notification_service
                        .notify(
                            "Review Automation",
                            &format!("Tests failed for task: {}", task.title),
                        )
                        .await;

                    return Ok(ReviewAction::TestFailed);
                }
                Err(e) => {
                    ReviewAutomationLog::create(
                        &self.db.pool,
                        task.id,
                        workspace.id,
                        ReviewAction::Error,
                        None,
                        Some(e.to_string()),
                    )
                    .await?;
                    return Err(e);
                }
            }
        }

        // Step 2: Auto-merge if enabled
        if settings.auto_merge_enabled {
            match self.attempt_auto_merge(task, workspace, workspace_path).await {
                Ok(()) => {
                    ReviewAutomationLog::create(
                        &self.db.pool,
                        task.id,
                        workspace.id,
                        ReviewAction::MergeCompleted,
                        None,
                        None,
                    )
                    .await?;

                    // Move task to done
                    Task::update_status(&self.db.pool, task.id, TaskStatus::Done).await?;

                    // Archive the workspace
                    Workspace::set_archived(&self.db.pool, workspace.id, true).await?;

                    self.notification_service
                        .notify(
                            "Review Automation",
                            &format!("Task completed: {}", task.title),
                        )
                        .await;

                    return Ok(ReviewAction::MergeCompleted);
                }
                Err(ReviewAutomationError::MergeConflict(msg)) => {
                    // Log the conflict with detailed information
                    ReviewAutomationLog::create(
                        &self.db.pool,
                        task.id,
                        workspace.id,
                        ReviewAction::MergeConflict,
                        None,
                        Some(format!(
                            "Merge conflict detected. Details: {}",
                            msg
                        )),
                    )
                    .await?;

                    // Check how many times this task has had merge conflicts
                    let conflict_count = ReviewAutomationLog::count_merge_conflicts(
                        &self.db.pool,
                        task.id,
                    )
                    .await?;

                    if conflict_count >= MAX_MERGE_CONFLICT_ATTEMPTS {
                        // Too many failures - cancel task and break it down into simpler subtasks
                        info!(
                            task_id = %task.id,
                            conflict_count = conflict_count,
                            "Review automation: max merge conflicts reached, cancelling and breaking down task"
                        );

                        // Cancel the original task
                        Task::update_status(&self.db.pool, task.id, TaskStatus::Cancelled).await?;

                        // Archive the workspace
                        Workspace::set_archived(&self.db.pool, workspace.id, true).await?;

                        // Try to break down the task into simpler subtasks
                        match self.breakdown_conflicting_task(&task, &msg).await {
                            Ok(subtask_count) => {
                                self.notification_service
                                    .notify(
                                        "Review Automation",
                                        &format!(
                                            "Task '{}' cancelled after {} merge conflicts. Created {} simpler subtasks.",
                                            task.title, conflict_count, subtask_count
                                        ),
                                    )
                                    .await;

                                ReviewAutomationLog::create(
                                    &self.db.pool,
                                    task.id,
                                    workspace.id,
                                    ReviewAction::Error,
                                    None,
                                    Some(format!(
                                        "Task cancelled after {} merge conflicts. Broken down into {} simpler subtasks.",
                                        conflict_count, subtask_count
                                    )),
                                )
                                .await?;
                            }
                            Err(e) => {
                                warn!(
                                    task_id = %task.id,
                                    error = %e,
                                    "Failed to break down conflicting task"
                                );

                                self.notification_service
                                    .notify(
                                        "Review Automation",
                                        &format!(
                                            "Task '{}' cancelled after {} merge conflicts. Manual breakdown required.",
                                            task.title, conflict_count
                                        ),
                                    )
                                    .await;
                            }
                        }

                        return Ok(ReviewAction::MergeConflict);
                    }

                    // Move task back to InProgress so the agent can resolve conflicts
                    // This mirrors what happens when user clicks "Resolve Conflicts"
                    Task::update_status(&self.db.pool, task.id, TaskStatus::InProgress).await?;

                    info!(
                        task_id = %task.id,
                        workspace_id = %workspace.id,
                        conflict_count = conflict_count,
                        "Review automation: merge conflict #{}, moved task back to InProgress",
                        conflict_count
                    );

                    self.notification_service
                        .notify(
                            "Review Automation",
                            &format!(
                                "Merge conflict #{} for '{}'. Task moved back to InProgress for conflict resolution. ({} attempts remaining)",
                                conflict_count, task.title, MAX_MERGE_CONFLICT_ATTEMPTS - conflict_count
                            ),
                        )
                        .await;

                    return Ok(ReviewAction::MergeConflict);
                }
                Err(e) => {
                    ReviewAutomationLog::create(
                        &self.db.pool,
                        task.id,
                        workspace.id,
                        ReviewAction::Error,
                        None,
                        Some(e.to_string()),
                    )
                    .await?;
                    return Err(e);
                }
            }
        }

        // If neither tests nor merge are enabled, just skip
        ReviewAutomationLog::create(
            &self.db.pool,
            task.id,
            workspace.id,
            ReviewAction::Skipped,
            None,
            Some("Neither tests nor auto-merge enabled".to_string()),
        )
        .await?;

        Ok(ReviewAction::Skipped)
    }

    /// Detect the project stack from files in the workspace
    fn detect_stack(&self, workspace_path: &str) -> ProjectStack {
        let path = Path::new(workspace_path);

        // Check for Node.js (package.json)
        if path.join("package.json").exists() {
            return ProjectStack::NodeJs;
        }

        // Check for Rust (Cargo.toml)
        if path.join("Cargo.toml").exists() {
            return ProjectStack::Rust;
        }

        // Check for Python (pyproject.toml, setup.py, or pytest.ini)
        if path.join("pyproject.toml").exists()
            || path.join("setup.py").exists()
            || path.join("pytest.ini").exists()
        {
            return ProjectStack::Python;
        }

        // Check for Go (go.mod)
        if path.join("go.mod").exists() {
            return ProjectStack::Go;
        }

        ProjectStack::Unknown
    }

    /// Run tests for a workspace
    async fn run_tests(
        &self,
        workspace: &Workspace,
        workspace_path: &str,
    ) -> Result<String, ReviewAutomationError> {
        // Detect the stack
        let stack = self.detect_stack(workspace_path);

        let (cmd, args) = match stack.test_command() {
            Some(c) => c,
            None => {
                info!(
                    workspace_id = %workspace.id,
                    "Review automation: unknown stack, skipping tests"
                );
                return Ok("Unknown stack, tests skipped".to_string());
            }
        };

        info!(
            workspace_id = %workspace.id,
            stack = ?stack,
            command = cmd,
            "Review automation: running tests"
        );

        let output = Command::new(cmd)
            .args(args)
            .current_dir(workspace_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ReviewAutomationError::CommandFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined_output = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);

        if output.status.success() {
            Ok(combined_output)
        } else {
            Err(ReviewAutomationError::TestFailed(combined_output))
        }
    }

    /// Attempt to auto-merge the workspace branch into target branches
    /// If the base branch has moved ahead, automatically rebase and retry
    async fn attempt_auto_merge(
        &self,
        task: &Task,
        workspace: &Workspace,
        workspace_path: &str,
    ) -> Result<(), ReviewAutomationError> {
        // Get workspace repos with their target branches
        let workspace_repos =
            WorkspaceRepo::find_repos_with_target_branch_for_workspace(&self.db.pool, workspace.id)
                .await?;

        if workspace_repos.is_empty() {
            warn!(
                workspace_id = %workspace.id,
                "Review automation: no repos found for workspace"
            );
            return Ok(());
        }

        // Merge each repo
        for repo_with_branch in &workspace_repos {
            let repo = &repo_with_branch.repo;
            let target_branch = &repo_with_branch.target_branch;
            let repo_path = &repo.path;

            // The workspace path is the container_ref, and each repo is in a subdirectory
            let task_worktree_path = Path::new(workspace_path).join(&repo.name);

            // Check if the worktree path exists
            if !task_worktree_path.exists() {
                warn!(
                    workspace_id = %workspace.id,
                    repo_id = %repo.id,
                    path = %task_worktree_path.display(),
                    "Review automation: worktree path does not exist"
                );
                continue;
            }

            info!(
                workspace_id = %workspace.id,
                repo_id = %repo.id,
                branch = %workspace.branch,
                target_branch = %target_branch,
                "Review automation: attempting merge"
            );

            // Perform the merge
            let commit_message = format!("Merge {} into {}\n\nTask: {}", workspace.branch, target_branch, task.title);

            let merge_result = self.git_service.merge_changes(
                repo_path,
                &task_worktree_path,
                &workspace.branch,
                target_branch,
                &commit_message,
            );

            match merge_result {
                Ok(merge_commit) => {
                    info!(
                        workspace_id = %workspace.id,
                        repo_id = %repo.id,
                        merge_commit = %merge_commit,
                        "Review automation: merge successful"
                    );

                    // Record the direct merge
                    Merge::create_direct(
                        &self.db.pool,
                        workspace.id,
                        repo.id,
                        target_branch,
                        &merge_commit,
                    )
                    .await?;
                }
                Err(super::git::GitServiceError::BranchesDiverged(_)) => {
                    // Base branch has moved ahead - try to rebase and merge
                    info!(
                        workspace_id = %workspace.id,
                        repo_id = %repo.id,
                        branch = %workspace.branch,
                        target_branch = %target_branch,
                        "Review automation: base branch diverged, attempting rebase"
                    );

                    // Get the fork point (old base) for rebase
                    let fork_point = match self.git_service.get_fork_point(
                        &task_worktree_path,
                        target_branch,
                        &workspace.branch,
                    ) {
                        Ok(fp) => fp,
                        Err(e) => {
                            return Err(ReviewAutomationError::MergeConflict(format!(
                                "Could not determine fork point for rebase: {}",
                                e
                            )));
                        }
                    };

                    // Attempt rebase onto new base
                    match self.git_service.rebase_branch(
                        repo_path,
                        &task_worktree_path,
                        target_branch,
                        &fork_point,
                        &workspace.branch,
                    ) {
                        Ok(new_head) => {
                            info!(
                                workspace_id = %workspace.id,
                                repo_id = %repo.id,
                                new_head = %new_head,
                                "Review automation: rebase successful, retrying merge"
                            );

                            // Retry the merge after successful rebase
                            match self.git_service.merge_changes(
                                repo_path,
                                &task_worktree_path,
                                &workspace.branch,
                                target_branch,
                                &commit_message,
                            ) {
                                Ok(merge_commit) => {
                                    info!(
                                        workspace_id = %workspace.id,
                                        repo_id = %repo.id,
                                        merge_commit = %merge_commit,
                                        "Review automation: merge successful after rebase"
                                    );

                                    Merge::create_direct(
                                        &self.db.pool,
                                        workspace.id,
                                        repo.id,
                                        target_branch,
                                        &merge_commit,
                                    )
                                    .await?;
                                }
                                Err(e) => {
                                    return Err(ReviewAutomationError::MergeConflict(format!(
                                        "Merge failed after rebase: {}",
                                        e
                                    )));
                                }
                            }
                        }
                        Err(super::git::GitServiceError::MergeConflicts(msg)) => {
                            // Rebase had conflicts - abort and report
                            let _ = self.git_service.abort_conflicts(&task_worktree_path);
                            return Err(ReviewAutomationError::MergeConflict(format!(
                                "Automatic rebase failed due to conflicts. Manual intervention required. {}",
                                msg
                            )));
                        }
                        Err(e) => {
                            // Rebase failed for other reasons - abort and report
                            let _ = self.git_service.abort_conflicts(&task_worktree_path);
                            return Err(ReviewAutomationError::MergeConflict(format!(
                                "Automatic rebase failed: {}",
                                e
                            )));
                        }
                    }
                }
                Err(super::git::GitServiceError::MergeConflicts(msg)) => {
                    return Err(ReviewAutomationError::MergeConflict(msg));
                }
                Err(e) => {
                    return Err(ReviewAutomationError::Git(e));
                }
            }
        }

        Ok(())
    }

    /// Get the current review automation status for a project
    pub async fn get_status(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<ReviewAutomationStatus, ReviewAutomationError> {
        let settings = ProjectReviewSettings::find_by_project_id(pool, project_id).await?;
        let latest_log = ReviewAutomationLog::find_latest_by_project_id(pool, project_id).await?;

        Ok(ReviewAutomationStatus {
            enabled: settings.as_ref().is_some_and(|s| s.enabled),
            auto_merge_enabled: settings.as_ref().is_some_and(|s| s.auto_merge_enabled),
            run_tests_enabled: settings.as_ref().is_some_and(|s| s.run_tests_enabled),
            last_action: latest_log.as_ref().map(|l| l.action.clone()),
            last_task_id: latest_log.map(|l| l.task_id),
        })
    }

    /// Enable review automation for a project
    pub async fn enable(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<ProjectReviewSettings, ReviewAutomationError> {
        Ok(ProjectReviewSettings::set_enabled(pool, project_id, true).await?)
    }

    /// Disable review automation for a project
    pub async fn disable(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<ProjectReviewSettings, ReviewAutomationError> {
        Ok(ProjectReviewSettings::set_enabled(pool, project_id, false).await?)
    }

    /// Get review automation logs for a project
    pub async fn get_logs(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i32,
    ) -> Result<Vec<ReviewAutomationLog>, ReviewAutomationError> {
        Ok(ReviewAutomationLog::find_by_project_id(pool, project_id, limit).await?)
    }

    /// Get review automation logs for a specific task
    pub async fn get_logs_by_task(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<ReviewAutomationLog>, ReviewAutomationError> {
        Ok(ReviewAutomationLog::find_by_task_id(pool, task_id).await?)
    }

    /// Break down a task that has failed to merge too many times into simpler subtasks
    async fn breakdown_conflicting_task(
        &self,
        task: &Task,
        conflict_details: &str,
    ) -> Result<usize, ReviewAutomationError> {
        let claude = ClaudeApiClient::from_env()
            .map_err(|e: ClaudeApiError| ReviewAutomationError::CommandFailed(e.to_string()))?;

        let prompt = format!(
            r#"A software development task has failed to merge {max_attempts} times due to conflicts.
The task needs to be broken down into smaller, simpler subtasks that are less likely to cause conflicts.

## Original Task
Title: {title}
Description: {description}
Layer: {layer}
Type: {task_type}

## Conflict Details
{conflict_details}

## Requirements
1. Break this task into 2-4 smaller, independent subtasks
2. Each subtask should be small enough to avoid merge conflicts
3. Subtasks should be able to be completed and merged independently
4. Focus on making atomic, isolated changes

## Output Format (JSON only):
{{
  "subtasks": [
    {{"title": "<subtask title>", "description": "<clear description of what to do>", "layer": "<data|backend|frontend|null>"}},
    ...
  ],
  "reasoning": "<brief explanation of how you split the task>"
}}"#,
            max_attempts = MAX_MERGE_CONFLICT_ATTEMPTS,
            title = task.title,
            description = task.description.as_deref().unwrap_or("(no description)"),
            layer = task.layer.as_ref().map(|l| l.to_string()).unwrap_or_else(|| "unspecified".to_string()),
            task_type = task.task_type.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "implementation".to_string()),
            conflict_details = conflict_details,
        );

        let system = Some(
            "You are a task breakdown assistant. Break complex tasks into smaller, independent pieces that can be merged without conflicts. Output valid JSON only.".to_string()
        );

        let response: ConflictBreakdownResponse = claude
            .ask_json::<ConflictBreakdownResponse>(&prompt, system)
            .await
            .map_err(|e: ClaudeApiError| ReviewAutomationError::CommandFailed(e.to_string()))?;

        if response.subtasks.is_empty() || response.subtasks.len() < 2 {
            return Err(ReviewAutomationError::CommandFailed(
                "AI did not suggest enough subtasks for breakdown".to_string(),
            ));
        }

        let mut created_count = 0;
        let base_sequence = task.sequence.unwrap_or(1);

        for (i, subtask) in response.subtasks.iter().enumerate() {
            let layer: Option<TaskLayer> = subtask.layer.as_ref().and_then(|l: &String| {
                match l.to_lowercase().as_str() {
                    "data" => Some(TaskLayer::Data),
                    "backend" => Some(TaskLayer::Backend),
                    "frontend" => Some(TaskLayer::Frontend),
                    "fullstack" => Some(TaskLayer::Fullstack),
                    "devops" => Some(TaskLayer::Devops),
                    "testing" => Some(TaskLayer::Testing),
                    _ => task.layer.clone(),
                }
            }).or_else(|| task.layer.clone());

            let create_task = CreateTask::subtask_of(
                task.project_id,
                subtask.title.clone(),
                Some(subtask.description.clone()),
                layer,
                task.task_type.clone(),
                base_sequence * 10 + (i as i32) + 1,
                task.testing_criteria.clone(),
                None,
                task.id,
            );

            match Task::create(&self.db.pool, &create_task, Uuid::new_v4()).await {
                Ok(new_task) => {
                    info!(
                        parent_task_id = %task.id,
                        subtask_id = %new_task.id,
                        subtask_title = %new_task.title,
                        "Review automation: created subtask from conflicting task"
                    );
                    created_count += 1;
                }
                Err(e) => {
                    warn!(
                        parent_task_id = %task.id,
                        error = %e,
                        "Failed to create subtask"
                    );
                }
            }
        }

        info!(
            task_id = %task.id,
            created_count = created_count,
            reasoning = %response.reasoning,
            "Review automation: task broken down after merge conflicts"
        );

        Ok(created_count)
    }
}
