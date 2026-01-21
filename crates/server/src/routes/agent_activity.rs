//! Routes for agent activity (autonomous task selection).

use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::agent_activity::{AgentActivityStatus, AgentTriggerResponse, ProjectAgentSettings};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::{
    agent_activity::AgentActivityService,
    container::ContainerService,
};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Response for enable/disable operations
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentActivitySettingsResponse {
    pub enabled: bool,
    pub interval_seconds: i32,
}

impl From<ProjectAgentSettings> for AgentActivitySettingsResponse {
    fn from(settings: ProjectAgentSettings) -> Self {
        Self {
            enabled: settings.enabled,
            interval_seconds: settings.interval_seconds,
        }
    }
}

/// Enable agent activity for a project
pub async fn enable_agent_activity(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<AgentActivitySettingsResponse>>, ApiError> {
    let settings = AgentActivityService::enable(&deployment.db().pool, project_id).await?;

    deployment
        .track_if_analytics_allowed(
            "agent_activity_enabled",
            serde_json::json!({
                "project_id": project_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(settings.into())))
}

/// Disable agent activity for a project
pub async fn disable_agent_activity(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<AgentActivitySettingsResponse>>, ApiError> {
    let settings = AgentActivityService::disable(&deployment.db().pool, project_id).await?;

    deployment
        .track_if_analytics_allowed(
            "agent_activity_disabled",
            serde_json::json!({
                "project_id": project_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(settings.into())))
}

/// Get agent activity status for a project
pub async fn get_agent_activity_status(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<AgentActivityStatus>>, ApiError> {
    let status = AgentActivityService::get_status(&deployment.db().pool, project_id).await?;
    Ok(ResponseJson(ApiResponse::success(status)))
}

/// Manually trigger agent activity to select next task
pub async fn trigger_agent_activity(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<AgentTriggerResponse>>, ApiError> {
    let notification_service = deployment.container().notification_service().clone();

    // Manual trigger doesn't use auto-attempt (user can start attempt separately)
    let response = AgentActivityService::check_and_select_next_task(
        &deployment.db().pool,
        &notification_service,
        project_id,
        None, // No auto-attempt for manual triggers
    )
    .await?;

    deployment
        .track_if_analytics_allowed(
            "agent_activity_triggered",
            serde_json::json!({
                "project_id": project_id.to_string(),
                "action": response.action.to_string(),
                "task_id": response.task_id.map(|id| id.to_string()),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(response)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/projects/{project_id}/agent-activity",
        Router::new()
            .route("/enable", post(enable_agent_activity))
            .route("/disable", post(disable_agent_activity))
            .route("/status", get(get_agent_activity_status))
            .route("/trigger", post(trigger_agent_activity)),
    )
}
