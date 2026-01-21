//! Routes for review automation (automatic testing and merging).

use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::review_automation::{
    ReviewAutomationLog, ReviewAutomationSettingsResponse, ReviewAutomationStatus,
};
use deployment::Deployment;
use services::services::review_automation::ReviewAutomationService;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// Enable review automation for a project
pub async fn enable_review_automation(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<ReviewAutomationSettingsResponse>>, ApiError> {
    let settings = ReviewAutomationService::enable(&deployment.db().pool, project_id).await?;

    deployment
        .track_if_analytics_allowed(
            "review_automation_enabled",
            serde_json::json!({
                "project_id": project_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(settings.into())))
}

/// Disable review automation for a project
pub async fn disable_review_automation(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<ReviewAutomationSettingsResponse>>, ApiError> {
    let settings = ReviewAutomationService::disable(&deployment.db().pool, project_id).await?;

    deployment
        .track_if_analytics_allowed(
            "review_automation_disabled",
            serde_json::json!({
                "project_id": project_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(settings.into())))
}

/// Get review automation status for a project
pub async fn get_review_automation_status(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<ReviewAutomationStatus>>, ApiError> {
    let status = ReviewAutomationService::get_status(&deployment.db().pool, project_id).await?;
    Ok(ResponseJson(ApiResponse::success(status)))
}

/// Get review automation logs for a project
pub async fn get_review_automation_logs(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<ReviewAutomationLog>>>, ApiError> {
    let logs = ReviewAutomationService::get_logs(&deployment.db().pool, project_id, 50).await?;
    Ok(ResponseJson(ApiResponse::success(logs)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/projects/{project_id}/review-automation",
        Router::new()
            .route("/enable", post(enable_review_automation))
            .route("/disable", post(disable_review_automation))
            .route("/status", get(get_review_automation_status))
            .route("/logs", get(get_review_automation_logs)),
    )
}
