use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use strum_macros::{Display, EnumString};
use ts_rs::TS;
use uuid::Uuid;

/// Action taken by the agent
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display)]
#[sqlx(type_name = "agent_action", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum AgentAction {
    Selected,
    Skipped,
    Error,
}

/// Agent activity settings for a project
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ProjectAgentSettings {
    pub id: Uuid,
    pub project_id: Uuid,
    pub enabled: bool,
    pub interval_seconds: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Log entry for agent activity
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AgentActivityLog {
    pub id: Uuid,
    pub project_id: Uuid,
    pub task_id: Option<Uuid>,
    pub action: AgentAction,
    pub reasoning: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Response for agent activity status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentActivityStatus {
    pub enabled: bool,
    pub interval_seconds: i32,
    pub last_run: Option<DateTime<Utc>>,
    pub last_selected_task_id: Option<Uuid>,
    pub last_reasoning: Option<String>,
}

/// Response for agent trigger action
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentTriggerResponse {
    pub action: AgentAction,
    pub task_id: Option<Uuid>,
    pub reasoning: Option<String>,
}

impl ProjectAgentSettings {
    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectAgentSettings,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                enabled as "enabled!: bool",
                interval_seconds as "interval_seconds!: i32",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM project_agent_settings
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
        interval_seconds: i32,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            ProjectAgentSettings,
            r#"INSERT INTO project_agent_settings (id, project_id, enabled, interval_seconds)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT(project_id) DO UPDATE SET
                enabled = excluded.enabled,
                interval_seconds = excluded.interval_seconds,
                updated_at = CURRENT_TIMESTAMP
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                enabled as "enabled!: bool",
                interval_seconds as "interval_seconds!: i32",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            enabled,
            interval_seconds
        )
        .fetch_one(pool)
        .await
    }

    pub async fn set_enabled(
        pool: &SqlitePool,
        project_id: Uuid,
        enabled: bool,
    ) -> Result<Self, sqlx::Error> {
        // Default interval is 60 seconds
        Self::create_or_update(pool, project_id, enabled, 60).await
    }

    pub async fn find_all_enabled(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectAgentSettings,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                enabled as "enabled!: bool",
                interval_seconds as "interval_seconds!: i32",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM project_agent_settings
            WHERE enabled = 1"#
        )
        .fetch_all(pool)
        .await
    }
}

impl AgentActivityLog {
    pub async fn create(
        pool: &SqlitePool,
        project_id: Uuid,
        task_id: Option<Uuid>,
        action: AgentAction,
        reasoning: Option<String>,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            AgentActivityLog,
            r#"INSERT INTO agent_activity_logs (id, project_id, task_id, action, reasoning)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                task_id as "task_id: Uuid",
                action as "action!: AgentAction",
                reasoning,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            project_id,
            task_id,
            action,
            reasoning
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_latest_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            AgentActivityLog,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                task_id as "task_id: Uuid",
                action as "action!: AgentAction",
                reasoning,
                created_at as "created_at!: DateTime<Utc>"
            FROM agent_activity_logs
            WHERE project_id = $1
            ORDER BY created_at DESC
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
            AgentActivityLog,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                task_id as "task_id: Uuid",
                action as "action!: AgentAction",
                reasoning,
                created_at as "created_at!: DateTime<Utc>"
            FROM agent_activity_logs
            WHERE project_id = $1
            ORDER BY created_at DESC
            LIMIT $2"#,
            project_id,
            limit
        )
        .fetch_all(pool)
        .await
    }
}
