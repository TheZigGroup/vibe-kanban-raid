//! Service for automated review processing: running tests and auto-merging branches.

use std::{path::Path, process::Stdio, time::Duration};

use db::{
    DBService,
    models::{
        merge::Merge,
        review_automation::{
            ProjectReviewSettings, ReviewAction, ReviewAutomationLog, ReviewAutomationStatus,
        },
        task::{Task, TaskStatus},
        workspace::Workspace,
        workspace_repo::WorkspaceRepo,
    },
};
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::{process::Command, time::interval};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{git::GitService, notification::NotificationService};

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
            poll_interval: Duration::from_secs(60), // Check every minute
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
                    ReviewAutomationLog::create(
                        &self.db.pool,
                        task.id,
                        workspace.id,
                        ReviewAction::MergeConflict,
                        None,
                        Some(msg.clone()),
                    )
                    .await?;

                    self.notification_service
                        .notify(
                            "Review Automation",
                            &format!("Merge conflict for task: {}", task.title),
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
                Err(super::git::GitServiceError::MergeConflicts(msg)) => {
                    return Err(ReviewAutomationError::MergeConflict(msg));
                }
                Err(super::git::GitServiceError::BranchesDiverged(msg)) => {
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
}
