use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Sqlite, SqlitePool, Type};
use strum_macros::{Display, EnumString};
use ts_rs::TS;
use uuid::Uuid;

use super::{project::Project, workspace::Workspace};

#[derive(
    Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display, Default,
)]
#[sqlx(type_name = "task_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    InReview,
    Done,
    Cancelled,
}

/// Source of task creation
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display, Default)]
#[sqlx(type_name = "task_source", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TaskSource {
    #[default]
    Manual,
    AiGenerated,
}

/// Layer/domain the task belongs to
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display)]
#[sqlx(type_name = "task_layer", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TaskLayer {
    Data,
    Backend,
    Frontend,
    Fullstack,
    Devops,
    Testing,
}

/// Type of task in architecture-first approach
#[derive(
    Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display, Default,
)]
#[sqlx(type_name = "task_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum TaskType {
    /// Architecture tasks: data models, API contracts, schemas
    Architecture,
    /// Implementation tasks: actual feature implementation
    #[default]
    Implementation,
    /// Integration tasks: wire all layers together
    Integration,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid, // Foreign key to Project
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub parent_workspace_id: Option<Uuid>, // Foreign key to parent Workspace
    pub source: TaskSource,
    pub layer: Option<TaskLayer>,
    pub task_type: Option<TaskType>,
    pub sequence: Option<i32>,
    pub testing_criteria: Option<String>,
    pub stage_started_at: Option<DateTime<Utc>>, // When task entered current status stage (for timeout detection)
    pub complexity_score: Option<i32>,            // AI-analyzed complexity (1-10)
    pub parent_task_id: Option<Uuid>,             // Link to parent task when broken down
    pub prevent_breakdown: bool,                  // Prevent automatic task breakdown
    pub post_task_actions: Option<String>,        // Instructions for updating .progress file
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskWithAttemptStatus {
    #[serde(flatten)]
    #[ts(flatten)]
    pub task: Task,
    pub has_in_progress_attempt: bool,
    pub last_attempt_failed: bool,
    pub executor: String,
}

impl std::ops::Deref for TaskWithAttemptStatus {
    type Target = Task;
    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

impl std::ops::DerefMut for TaskWithAttemptStatus {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.task
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskRelationships {
    pub parent_task: Option<Task>, // The task that owns the parent workspace
    pub current_workspace: Workspace, // The workspace we're viewing
    pub children: Vec<Task>,       // Tasks created from this workspace
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateTask {
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub parent_workspace_id: Option<Uuid>,
    pub image_ids: Option<Vec<Uuid>>,
    pub source: Option<TaskSource>,
    pub layer: Option<TaskLayer>,
    pub task_type: Option<TaskType>,
    pub sequence: Option<i32>,
    pub testing_criteria: Option<String>,
    pub parent_task_id: Option<Uuid>,    // Link to parent task when broken down
    pub prevent_breakdown: Option<bool>, // Prevent automatic task breakdown
    pub post_task_actions: Option<String>, // Instructions for updating .progress file
}

impl CreateTask {
    pub fn from_title_description(
        project_id: Uuid,
        title: String,
        description: Option<String>,
    ) -> Self {
        Self {
            project_id,
            title,
            description,
            status: Some(TaskStatus::Todo),
            parent_workspace_id: None,
            image_ids: None,
            source: None,
            layer: None,
            task_type: None,
            sequence: None,
            testing_criteria: None,
            parent_task_id: None,
            prevent_breakdown: None,
            post_task_actions: None,
        }
    }

    /// Create a task that was AI-generated from requirements
    pub fn ai_generated(
        project_id: Uuid,
        title: String,
        description: Option<String>,
        layer: Option<TaskLayer>,
        task_type: Option<TaskType>,
        sequence: i32,
        testing_criteria: Option<String>,
        post_task_actions: Option<String>,
    ) -> Self {
        Self {
            project_id,
            title,
            description,
            status: Some(TaskStatus::Todo),
            parent_workspace_id: None,
            image_ids: None,
            source: Some(TaskSource::AiGenerated),
            layer,
            task_type,
            sequence: Some(sequence),
            testing_criteria,
            parent_task_id: None,
            prevent_breakdown: None,
            post_task_actions,
        }
    }

    /// Create a subtask broken down from a complex parent task
    /// Subtasks automatically have prevent_breakdown=true to avoid recursive breakdown
    pub fn subtask_of(
        project_id: Uuid,
        title: String,
        description: Option<String>,
        layer: Option<TaskLayer>,
        task_type: Option<TaskType>,
        sequence: i32,
        testing_criteria: Option<String>,
        post_task_actions: Option<String>,
        parent_task_id: Uuid,
    ) -> Self {
        Self {
            project_id,
            title,
            description,
            status: Some(TaskStatus::Todo),
            parent_workspace_id: None,
            image_ids: None,
            source: Some(TaskSource::AiGenerated),
            layer,
            task_type,
            sequence: Some(sequence),
            testing_criteria,
            parent_task_id: Some(parent_task_id),
            prevent_breakdown: Some(true), // Subtasks should not be broken down further
            post_task_actions,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub parent_workspace_id: Option<Uuid>,
    pub image_ids: Option<Vec<Uuid>>,
}

impl Task {
    pub fn to_prompt(&self) -> String {
        if let Some(description) = self.description.as_ref().filter(|d| !d.trim().is_empty()) {
            format!("{}\n\n{}", &self.title, description)
        } else {
            self.title.clone()
        }
    }

    pub async fn parent_project(&self, pool: &SqlitePool) -> Result<Option<Project>, sqlx::Error> {
        Project::find_by_id(pool, self.project_id).await
    }

    pub async fn find_by_project_id_with_attempt_status(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<TaskWithAttemptStatus>, sqlx::Error> {
        let records = sqlx::query!(
            r#"SELECT
  t.id                            AS "id!: Uuid",
  t.project_id                    AS "project_id!: Uuid",
  t.title,
  t.description,
  t.status                        AS "status!: TaskStatus",
  t.parent_workspace_id           AS "parent_workspace_id: Uuid",
  t.source                        AS "source!: TaskSource",
  t.layer                         AS "layer: TaskLayer",
  t.task_type                     AS "task_type: TaskType",
  t.sequence                      AS "sequence: i32",
  t.testing_criteria,
  t.stage_started_at              AS "stage_started_at: DateTime<Utc>",
  t.complexity_score              AS "complexity_score: i32",
  t.parent_task_id                AS "parent_task_id: Uuid",
  t.prevent_breakdown             AS "prevent_breakdown!: i64",
  t.post_task_actions,
  t.created_at                    AS "created_at!: DateTime<Utc>",
  t.updated_at                    AS "updated_at!: DateTime<Utc>",

  CASE WHEN EXISTS (
    SELECT 1
      FROM workspaces w
      JOIN sessions s ON s.workspace_id = w.id
      JOIN execution_processes ep ON ep.session_id = s.id
     WHERE w.task_id       = t.id
       AND ep.status        = 'running'
       AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     LIMIT 1
  ) THEN 1 ELSE 0 END            AS "has_in_progress_attempt!: i64",

  CASE WHEN (
    SELECT ep.status
      FROM workspaces w
      JOIN sessions s ON s.workspace_id = w.id
      JOIN execution_processes ep ON ep.session_id = s.id
     WHERE w.task_id       = t.id
     AND ep.run_reason IN ('setupscript','cleanupscript','codingagent')
     ORDER BY ep.created_at DESC
     LIMIT 1
  ) IN ('failed','killed') THEN 1 ELSE 0 END
                                 AS "last_attempt_failed!: i64",

  ( SELECT s.executor
      FROM workspaces w
      JOIN sessions s ON s.workspace_id = w.id
      WHERE w.task_id = t.id
     ORDER BY s.created_at DESC
      LIMIT 1
    )                               AS "executor!: String"

FROM tasks t
WHERE t.project_id = $1
ORDER BY t.created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await?;

        let tasks = records
            .into_iter()
            .map(|rec| TaskWithAttemptStatus {
                task: Task {
                    id: rec.id,
                    project_id: rec.project_id,
                    title: rec.title,
                    description: rec.description,
                    status: rec.status,
                    parent_workspace_id: rec.parent_workspace_id,
                    source: rec.source,
                    layer: rec.layer,
                    task_type: rec.task_type,
                    sequence: rec.sequence,
                    testing_criteria: rec.testing_criteria,
                    stage_started_at: rec.stage_started_at,
                    complexity_score: rec.complexity_score,
                    parent_task_id: rec.parent_task_id,
                    prevent_breakdown: rec.prevent_breakdown != 0,
                    post_task_actions: rec.post_task_actions,
                    created_at: rec.created_at,
                    updated_at: rec.updated_at,
                },
                has_in_progress_attempt: rec.has_in_progress_attempt != 0,
                last_attempt_failed: rec.last_attempt_failed != 0,
                executor: rec.executor,
            })
            .collect();

        Ok(tasks)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", source as "source!: TaskSource", layer as "layer: TaskLayer", task_type as "task_type: TaskType", sequence as "sequence: i32", testing_criteria, stage_started_at as "stage_started_at: DateTime<Utc>", complexity_score as "complexity_score: i32", parent_task_id as "parent_task_id: Uuid", prevent_breakdown as "prevent_breakdown!: bool", post_task_actions, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_rowid(pool: &SqlitePool, rowid: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", source as "source!: TaskSource", layer as "layer: TaskLayer", task_type as "task_type: TaskType", sequence as "sequence: i32", testing_criteria, stage_started_at as "stage_started_at: DateTime<Utc>", complexity_score as "complexity_score: i32", parent_task_id as "parent_task_id: Uuid", prevent_breakdown as "prevent_breakdown!: bool", post_task_actions, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE rowid = $1"#,
            rowid
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreateTask,
        task_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let status = data.status.clone().unwrap_or_default();
        let source = data.source.clone().unwrap_or_default();
        let prevent_breakdown = data.prevent_breakdown.unwrap_or(false);
        sqlx::query_as!(
            Task,
            r#"INSERT INTO tasks (id, project_id, title, description, status, parent_workspace_id, source, layer, task_type, sequence, testing_criteria, parent_task_id, prevent_breakdown, post_task_actions)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", source as "source!: TaskSource", layer as "layer: TaskLayer", task_type as "task_type: TaskType", sequence as "sequence: i32", testing_criteria, stage_started_at as "stage_started_at: DateTime<Utc>", complexity_score as "complexity_score: i32", parent_task_id as "parent_task_id: Uuid", prevent_breakdown as "prevent_breakdown!: bool", post_task_actions, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            task_id,
            data.project_id,
            data.title,
            data.description,
            status,
            data.parent_workspace_id,
            source,
            data.layer,
            data.task_type,
            data.sequence,
            data.testing_criteria,
            data.parent_task_id,
            prevent_breakdown,
            data.post_task_actions
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        project_id: Uuid,
        title: String,
        description: Option<String>,
        status: TaskStatus,
        parent_workspace_id: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"UPDATE tasks
               SET title = $3, description = $4, status = $5, parent_workspace_id = $6
               WHERE id = $1 AND project_id = $2
               RETURNING id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", source as "source!: TaskSource", layer as "layer: TaskLayer", task_type as "task_type: TaskType", sequence as "sequence: i32", testing_criteria, stage_started_at as "stage_started_at: DateTime<Utc>", complexity_score as "complexity_score: i32", parent_task_id as "parent_task_id: Uuid", prevent_breakdown as "prevent_breakdown!: bool", post_task_actions, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            title,
            description,
            status,
            parent_workspace_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: TaskStatus,
    ) -> Result<(), sqlx::Error> {
        // Set stage_started_at when entering InProgress or InReview, clear it otherwise
        let should_set_stage_time =
            matches!(status, TaskStatus::InProgress | TaskStatus::InReview);

        if should_set_stage_time {
            sqlx::query!(
                "UPDATE tasks SET status = $2, stage_started_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
                id,
                status
            )
            .execute(pool)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE tasks SET status = $2, stage_started_at = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
                id,
                status
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    /// Find tasks that have been stalled in a given status for longer than the timeout
    pub async fn find_stalled_tasks(
        pool: &SqlitePool,
        project_id: Uuid,
        status: TaskStatus,
        timeout_minutes: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let timeout_str = format!("-{} minutes", timeout_minutes);
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", source as "source!: TaskSource", layer as "layer: TaskLayer", task_type as "task_type: TaskType", sequence as "sequence: i32", testing_criteria, stage_started_at as "stage_started_at: DateTime<Utc>", complexity_score as "complexity_score: i32", parent_task_id as "parent_task_id: Uuid", prevent_breakdown as "prevent_breakdown!: bool", post_task_actions, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE project_id = $1
                 AND status = $2
                 AND stage_started_at IS NOT NULL
                 AND datetime(stage_started_at) < datetime('now', $3)
               ORDER BY stage_started_at ASC"#,
            project_id,
            status,
            timeout_str
        )
        .fetch_all(pool)
        .await
    }

    /// Update the complexity score for a task
    pub async fn update_complexity_score(
        pool: &SqlitePool,
        id: Uuid,
        complexity_score: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE tasks SET complexity_score = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
            id,
            complexity_score
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update the parent_task_id field for a task (for subtask linking)
    pub async fn update_parent_task_id(
        pool: &SqlitePool,
        task_id: Uuid,
        parent_task_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE tasks SET parent_task_id = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
            task_id,
            parent_task_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find subtasks of a parent task
    pub async fn find_subtasks(pool: &SqlitePool, parent_task_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", source as "source!: TaskSource", layer as "layer: TaskLayer", task_type as "task_type: TaskType", sequence as "sequence: i32", testing_criteria, stage_started_at as "stage_started_at: DateTime<Utc>", complexity_score as "complexity_score: i32", parent_task_id as "parent_task_id: Uuid", prevent_breakdown as "prevent_breakdown!: bool", post_task_actions, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE parent_task_id = $1
               ORDER BY sequence ASC, created_at ASC"#,
            parent_task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Update the parent_workspace_id field for a task
    pub async fn update_parent_workspace_id(
        pool: &SqlitePool,
        task_id: Uuid,
        parent_workspace_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE tasks SET parent_workspace_id = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
            task_id,
            parent_workspace_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Nullify parent_workspace_id for all tasks that reference the given workspace ID
    /// This breaks parent-child relationships before deleting a parent task
    pub async fn nullify_children_by_workspace_id<'e, E>(
        executor: E,
        workspace_id: Uuid,
    ) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result = sqlx::query!(
            "UPDATE tasks SET parent_workspace_id = NULL WHERE parent_workspace_id = $1",
            workspace_id
        )
        .execute(executor)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete<'e, E>(executor: E, id: Uuid) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result = sqlx::query!("DELETE FROM tasks WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn find_children_by_workspace_id(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // Find only child tasks that have this workspace as their parent
        sqlx::query_as!(
            Task,
            r#"SELECT id as "id!: Uuid", project_id as "project_id!: Uuid", title, description, status as "status!: TaskStatus", parent_workspace_id as "parent_workspace_id: Uuid", source as "source!: TaskSource", layer as "layer: TaskLayer", task_type as "task_type: TaskType", sequence as "sequence: i32", testing_criteria, stage_started_at as "stage_started_at: DateTime<Utc>", complexity_score as "complexity_score: i32", parent_task_id as "parent_task_id: Uuid", prevent_breakdown as "prevent_breakdown!: bool", post_task_actions, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE parent_workspace_id = $1
               ORDER BY created_at DESC"#,
            workspace_id,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_relationships_for_workspace(
        pool: &SqlitePool,
        workspace: &Workspace,
    ) -> Result<TaskRelationships, sqlx::Error> {
        // 1. Get the current task (task that owns this workspace)
        let current_task = Self::find_by_id(pool, workspace.task_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // 2. Get parent task (if current task was created by another workspace)
        let parent_task = if let Some(parent_workspace_id) = current_task.parent_workspace_id {
            // Find the workspace that created the current task
            if let Ok(Some(parent_workspace)) =
                Workspace::find_by_id(pool, parent_workspace_id).await
            {
                // Find the task that owns that parent workspace - THAT's the real parent
                Self::find_by_id(pool, parent_workspace.task_id).await?
            } else {
                None
            }
        } else {
            None
        };

        // 3. Get children tasks (created from this workspace)
        let children = Self::find_children_by_workspace_id(pool, workspace.id).await?;

        Ok(TaskRelationships {
            parent_task,
            current_workspace: workspace.clone(),
            children,
        })
    }

    /// Find tasks in "inreview" status that have completed attempts (no running processes).
    /// Returns (task, workspace) pairs for each eligible task.
    pub async fn find_in_review_with_completed_attempts(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<(Task, Workspace)>, sqlx::Error> {
        // Find tasks in review status that:
        // 1. Have at least one workspace
        // 2. Have no currently running execution processes
        // 3. Have at least one completed execution process (to ensure work was done)
        let records = sqlx::query!(
            r#"SELECT
                t.id as "task_id!: Uuid",
                t.project_id as "task_project_id!: Uuid",
                t.title as "task_title!",
                t.description as "task_description",
                t.status as "task_status!: TaskStatus",
                t.parent_workspace_id as "task_parent_workspace_id: Uuid",
                t.source as "task_source!: TaskSource",
                t.layer as "task_layer: TaskLayer",
                t.task_type as "task_task_type: TaskType",
                t.sequence as "task_sequence: i32",
                t.testing_criteria as "task_testing_criteria",
                t.stage_started_at as "task_stage_started_at: DateTime<Utc>",
                t.complexity_score as "task_complexity_score: i32",
                t.parent_task_id as "task_parent_task_id: Uuid",
                t.prevent_breakdown as "task_prevent_breakdown!: bool",
                t.post_task_actions as "task_post_task_actions",
                t.created_at as "task_created_at!: DateTime<Utc>",
                t.updated_at as "task_updated_at!: DateTime<Utc>",
                w.id as "workspace_id!: Uuid",
                w.task_id as "workspace_task_id!: Uuid",
                w.container_ref as "workspace_container_ref",
                w.branch as "workspace_branch!",
                w.agent_working_dir as "workspace_agent_working_dir",
                w.setup_completed_at as "workspace_setup_completed_at: DateTime<Utc>",
                w.created_at as "workspace_created_at!: DateTime<Utc>",
                w.updated_at as "workspace_updated_at!: DateTime<Utc>",
                w.archived as "workspace_archived!: bool",
                w.pinned as "workspace_pinned!: bool",
                w.name as "workspace_name"
            FROM tasks t
            JOIN workspaces w ON w.task_id = t.id
            WHERE t.project_id = $1
              AND t.status = 'inreview'
              AND w.archived = 0
              -- Has at least one completed execution process (codingagent)
              AND EXISTS (
                  SELECT 1
                  FROM sessions s
                  JOIN execution_processes ep ON ep.session_id = s.id
                  WHERE s.workspace_id = w.id
                    AND ep.run_reason = 'codingagent'
                    AND ep.status IN ('completed', 'failed', 'killed')
              )
              -- No running execution processes
              AND NOT EXISTS (
                  SELECT 1
                  FROM sessions s
                  JOIN execution_processes ep ON ep.session_id = s.id
                  WHERE s.workspace_id = w.id
                    AND ep.status = 'running'
              )
            ORDER BY t.created_at ASC
            LIMIT 1"#,
            project_id
        )
        .fetch_all(pool)
        .await?;

        let result = records
            .into_iter()
            .map(|rec| {
                let task = Task {
                    id: rec.task_id,
                    project_id: rec.task_project_id,
                    title: rec.task_title,
                    description: rec.task_description,
                    status: rec.task_status,
                    parent_workspace_id: rec.task_parent_workspace_id,
                    source: rec.task_source,
                    layer: rec.task_layer,
                    task_type: rec.task_task_type,
                    sequence: rec.task_sequence,
                    testing_criteria: rec.task_testing_criteria,
                    stage_started_at: rec.task_stage_started_at,
                    complexity_score: rec.task_complexity_score,
                    parent_task_id: rec.task_parent_task_id,
                    prevent_breakdown: rec.task_prevent_breakdown,
                    post_task_actions: rec.task_post_task_actions,
                    created_at: rec.task_created_at,
                    updated_at: rec.task_updated_at,
                };
                let workspace = Workspace {
                    id: rec.workspace_id,
                    task_id: rec.workspace_task_id,
                    container_ref: rec.workspace_container_ref,
                    branch: rec.workspace_branch,
                    agent_working_dir: rec.workspace_agent_working_dir,
                    setup_completed_at: rec.workspace_setup_completed_at,
                    created_at: rec.workspace_created_at,
                    updated_at: rec.workspace_updated_at,
                    archived: rec.workspace_archived,
                    pinned: rec.workspace_pinned,
                    name: rec.workspace_name,
                };
                (task, workspace)
            })
            .collect();

        Ok(result)
    }
}
