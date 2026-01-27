//! Service for detecting and handling stalled tasks that have exceeded timeout thresholds.

use std::time::Duration;

use db::{
    DBService,
    models::{
        agent_activity::{AgentAction, AgentActivityLog},
        execution_process::{ExecutionProcess, ExecutionProcessStatus},
        task::{Task, TaskStatus},
    },
};
use thiserror::Error;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::notification::NotificationService;

#[derive(Debug, Error)]
pub enum TaskTimeoutError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Background service for detecting and handling stalled tasks
pub struct TaskTimeoutService {
    db: DBService,
    notification_service: NotificationService,
    poll_interval: Duration,
    in_progress_timeout_minutes: i64,
    in_review_timeout_minutes: i64,
}

impl TaskTimeoutService {
    /// Spawn the background task timeout service
    pub async fn spawn(
        db: DBService,
        notification_service: NotificationService,
    ) -> tokio::task::JoinHandle<()> {
        let service = Self {
            db,
            notification_service,
            poll_interval: Duration::from_secs(10), // Check every 10 seconds
            in_progress_timeout_minutes: 20,        // 20 minute timeout for in-progress
            in_review_timeout_minutes: 20,          // 20 minute timeout for in-review
        };
        tokio::spawn(async move {
            service.start().await;
        })
    }

    async fn start(&self) {
        info!(
            "Starting task timeout service with interval {:?}, in_progress timeout: {} min, in_review timeout: {} min",
            self.poll_interval, self.in_progress_timeout_minutes, self.in_review_timeout_minutes
        );

        let mut interval = interval(self.poll_interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.check_for_stalled_tasks().await {
                error!("Error checking for stalled tasks: {}", e);
            }
        }
    }

    /// Check for stalled tasks across all projects
    async fn check_for_stalled_tasks(&self) -> Result<(), TaskTimeoutError> {
        // Get all unique project IDs that have active tasks
        let project_ids = self.get_projects_with_active_tasks().await?;

        if project_ids.is_empty() {
            debug!("Task timeout: no projects with active tasks");
            return Ok(());
        }

        for project_id in project_ids {
            // Check in-progress tasks
            if let Err(e) = self
                .process_stalled_tasks(
                    project_id,
                    TaskStatus::InProgress,
                    self.in_progress_timeout_minutes,
                )
                .await
            {
                warn!(
                    project_id = %project_id,
                    error = %e,
                    "Error processing stalled in-progress tasks"
                );
            }

            // Check in-review tasks
            if let Err(e) = self
                .process_stalled_tasks(
                    project_id,
                    TaskStatus::InReview,
                    self.in_review_timeout_minutes,
                )
                .await
            {
                warn!(
                    project_id = %project_id,
                    error = %e,
                    "Error processing stalled in-review tasks"
                );
            }
        }

        Ok(())
    }

    /// Get all project IDs that have tasks in InProgress or InReview status
    async fn get_projects_with_active_tasks(&self) -> Result<Vec<Uuid>, TaskTimeoutError> {
        let project_ids: Vec<(Uuid,)> = sqlx::query_as(
            r#"SELECT DISTINCT project_id
               FROM tasks
               WHERE status IN ('inprogress', 'inreview')
                 AND stage_started_at IS NOT NULL"#,
        )
        .fetch_all(&self.db.pool)
        .await?;

        Ok(project_ids.into_iter().map(|(id,)| id).collect())
    }

    /// Process stalled tasks for a specific project and status
    async fn process_stalled_tasks(
        &self,
        project_id: Uuid,
        status: TaskStatus,
        timeout_minutes: i64,
    ) -> Result<(), TaskTimeoutError> {
        let stalled_tasks =
            Task::find_stalled_tasks(&self.db.pool, project_id, status.clone(), timeout_minutes)
                .await?;

        for task in stalled_tasks {
            info!(
                task_id = %task.id,
                project_id = %project_id,
                status = %status,
                stage_started_at = ?task.stage_started_at,
                "Task timeout: found stalled task, cancelling"
            );

            // Mark any running processes as killed
            if let Err(e) = self.mark_task_processes_killed(task.id).await {
                warn!(
                    task_id = %task.id,
                    error = %e,
                    "Task timeout: error marking processes as killed"
                );
            }

            // Cancel the task
            Task::update_status(&self.db.pool, task.id, TaskStatus::Cancelled).await?;

            // Log the timeout action
            AgentActivityLog::create(
                &self.db.pool,
                project_id,
                Some(task.id),
                AgentAction::Timeout,
                Some(format!(
                    "Task cancelled due to {} minute timeout in {} status",
                    timeout_minutes, status
                )),
            )
            .await?;

            // Send notification
            self.notification_service
                .notify(
                    "Task Timeout",
                    &format!(
                        "Task '{}' cancelled due to timeout ({}+ minutes in {} status)",
                        task.title, timeout_minutes, status
                    ),
                )
                .await;
        }

        Ok(())
    }

    /// Mark all running execution processes associated with a task as killed
    async fn mark_task_processes_killed(&self, task_id: Uuid) -> Result<(), TaskTimeoutError> {
        // Find all running processes for workspaces belonging to this task
        let running_process_ids: Vec<(Uuid,)> = sqlx::query_as(
            r#"SELECT ep.id
            FROM execution_processes ep
            JOIN sessions s ON ep.session_id = s.id
            JOIN workspaces w ON s.workspace_id = w.id
            WHERE w.task_id = $1
              AND ep.status = 'running'"#,
        )
        .bind(task_id)
        .fetch_all(&self.db.pool)
        .await?;

        for (process_id,) in running_process_ids {
            info!(
                task_id = %task_id,
                process_id = %process_id,
                "Task timeout: marking process as killed"
            );

            // Mark the process as killed (the container service will handle actual termination)
            ExecutionProcess::update_completion(
                &self.db.pool,
                process_id,
                ExecutionProcessStatus::Killed,
                None,
            )
            .await?;
        }

        Ok(())
    }
}
