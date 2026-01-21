use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::post,
};
use db::models::project_requirements::{CreateProjectRequirements, ProjectRequirementsStatus};
use deployment::Deployment;
use services::services::requirements_analyzer::RequirementsAnalyzer;
use uuid::Uuid;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

/// POST /api/projects/{project_id}/requirements
/// Create requirements and start AI analysis
pub async fn create_requirements(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    axum::Json(payload): axum::Json<CreateProjectRequirements>,
) -> Result<ResponseJson<ApiResponse<ProjectRequirementsStatus>>, ApiError> {
    let analyzer = RequirementsAnalyzer::new(deployment.db().pool.clone())?;

    let requirements = analyzer.create_and_analyze(project_id, payload).await?;

    let status = ProjectRequirementsStatus {
        id: requirements.id,
        project_id: requirements.project_id,
        generation_status: requirements.generation_status.clone(),
        analysis_result: requirements.parsed_analysis(),
        tasks_generated: None,
        error_message: requirements.error_message,
        created_at: requirements.created_at,
        updated_at: requirements.updated_at,
    };

    deployment
        .track_if_analytics_allowed(
            "requirements_created",
            serde_json::json!({
                "project_id": project_id.to_string(),
                "requirements_id": status.id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(status)))
}

/// GET /api/projects/{project_id}/requirements
/// Get requirements status and analysis result
pub async fn get_requirements(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Option<ProjectRequirementsStatus>>>, ApiError> {
    let analyzer = RequirementsAnalyzer::new(deployment.db().pool.clone())?;

    let requirements = analyzer.get_status(project_id).await?;

    let status = requirements.map(|req| {
        // Count generated tasks
        let tasks_generated = None; // Could query tasks table if needed

        ProjectRequirementsStatus {
            id: req.id,
            project_id: req.project_id,
            generation_status: req.generation_status.clone(),
            analysis_result: req.parsed_analysis(),
            tasks_generated,
            error_message: req.error_message,
            created_at: req.created_at,
            updated_at: req.updated_at,
        }
    });

    Ok(ResponseJson(ApiResponse::success(status)))
}

/// DELETE /api/projects/{project_id}/requirements
/// Delete requirements and optionally the generated tasks
pub async fn delete_requirements(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let analyzer = RequirementsAnalyzer::new(deployment.db().pool.clone())?;

    // Delete requirements and associated AI-generated tasks
    analyzer.delete(project_id, true).await?;

    deployment
        .track_if_analytics_allowed(
            "requirements_deleted",
            serde_json::json!({
                "project_id": project_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/projects/{project_id}/requirements",
        Router::new().route("/", post(create_requirements).get(get_requirements).delete(delete_requirements)),
    )
}
