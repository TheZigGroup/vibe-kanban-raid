use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use strum_macros::{Display, EnumString};
use ts_rs::TS;
use uuid::Uuid;

/// Action taken by the review automation service
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display)]
#[sqlx(type_name = "review_action", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReviewAction {
    TestPassed,
    TestFailed,
    MergeCompleted,
    MergeConflict,
    Skipped,
    Error,
}

/// Review automation settings for a project
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ProjectReviewSettings {
    pub id: Uuid,
    pub project_id: Uuid,
    pub enabled: bool,
    pub auto_merge_enabled: bool,
    pub run_tests_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Log entry for review automation activity
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ReviewAutomationLog {
    pub id: Uuid,
    pub task_id: Uuid,
    pub workspace_id: Uuid,
    pub action: ReviewAction,
    pub output: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Response for review automation status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ReviewAutomationStatus {
    pub enabled: bool,
    pub auto_merge_enabled: bool,
    pub run_tests_enabled: bool,
    pub last_action: Option<ReviewAction>,
    pub last_task_id: Option<Uuid>,
}

/// Response for enable/disable operations
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ReviewAutomationSettingsResponse {
    pub enabled: bool,
    pub auto_merge_enabled: bool,
    pub run_tests_enabled: bool,
}

impl From<ProjectReviewSettings> for ReviewAutomationSettingsResponse {
    fn from(settings: ProjectReviewSettings) -> Self {
        Self {
            enabled: settings.enabled,
            auto_merge_enabled: settings.auto_merge_enabled,
            run_tests_enabled: settings.run_tests_enabled,
        }
    }
}

impl ProjectReviewSettings {
    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectReviewSettings,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                enabled as "enabled!: bool",
                auto_merge_enabled as "auto_merge_enabled!: bool",
                run_tests_enabled as "run_tests_enabled!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM project_review_settings
            WHERE project_id = $1"#,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create_or_update(
        pool: &SqlitePool,
        project_id: Uuid,
        enabled: bool,
        auto_merge_enabled: bool,
        run_tests_enabled: bool,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            ProjectReviewSettings,
            r#"INSERT INTO project_review_settings (id, project_id, enabled, auto_merge_enabled, run_tests_enabled)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT(project_id) DO UPDATE SET
                enabled = excluded.enabled,
                auto_merge_enabled = excluded.auto_merge_enabled,
                run_tests_enabled = excluded.run_tests_enabled,
                updated_at = datetime('now', 'subsec')
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                enabled as "enabled!: bool",
                auto_merge_enabled as "auto_merge_enabled!: bool",
                run_tests_enabled as "run_tests_enabled!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            enabled,
            auto_merge_enabled,
            run_tests_enabled
        )
        .fetch_one(pool)
        .await
    }

    pub async fn set_enabled(
        pool: &SqlitePool,
        project_id: Uuid,
        enabled: bool,
    ) -> Result<Self, sqlx::Error> {
        // Default: auto_merge and run_tests are enabled
        Self::create_or_update(pool, project_id, enabled, true, true).await
    }

    pub async fn find_all_enabled(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectReviewSettings,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                enabled as "enabled!: bool",
                auto_merge_enabled as "auto_merge_enabled!: bool",
                run_tests_enabled as "run_tests_enabled!: bool",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM project_review_settings
            WHERE enabled = 1"#
        )
        .fetch_all(pool)
        .await
    }
}

impl ReviewAutomationLog {
    pub async fn create(
        pool: &SqlitePool,
        task_id: Uuid,
        workspace_id: Uuid,
        action: ReviewAction,
        output: Option<String>,
        error_message: Option<String>,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            ReviewAutomationLog,
            r#"INSERT INTO review_automation_logs (id, task_id, workspace_id, action, output, error_message)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                workspace_id as "workspace_id!: Uuid",
                action as "action!: ReviewAction",
                output,
                error_message,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            task_id,
            workspace_id,
            action,
            output,
            error_message
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_latest_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ReviewAutomationLog,
            r#"SELECT
                ral.id as "id!: Uuid",
                ral.task_id as "task_id!: Uuid",
                ral.workspace_id as "workspace_id!: Uuid",
                ral.action as "action!: ReviewAction",
                ral.output,
                ral.error_message,
                ral.created_at as "created_at!: DateTime<Utc>"
            FROM review_automation_logs ral
            JOIN tasks t ON ral.task_id = t.id
            WHERE t.project_id = $1
            ORDER BY ral.created_at DESC
            LIMIT 1"#,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i32,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ReviewAutomationLog,
            r#"SELECT
                ral.id as "id!: Uuid",
                ral.task_id as "task_id!: Uuid",
                ral.workspace_id as "workspace_id!: Uuid",
                ral.action as "action!: ReviewAction",
                ral.output,
                ral.error_message,
                ral.created_at as "created_at!: DateTime<Utc>"
            FROM review_automation_logs ral
            JOIN tasks t ON ral.task_id = t.id
            WHERE t.project_id = $1
            ORDER BY ral.created_at DESC
            LIMIT $2"#,
            project_id,
            limit
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_task_id(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ReviewAutomationLog,
            r#"SELECT
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                workspace_id as "workspace_id!: Uuid",
                action as "action!: ReviewAction",
                output,
                error_message,
                created_at as "created_at!: DateTime<Utc>"
            FROM review_automation_logs
            WHERE task_id = $1
            ORDER BY created_at DESC"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Count the number of merge conflict attempts for a task
    pub async fn count_merge_conflicts(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64"
            FROM review_automation_logs
            WHERE task_id = $1 AND action = 'merge_conflict'"#,
            task_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result)
    }
}
