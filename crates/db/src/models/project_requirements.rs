use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use strum_macros::{Display, EnumString};
use ts_rs::TS;
use uuid::Uuid;

/// Status of requirements analysis and task generation
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display, Default)]
#[sqlx(type_name = "generation_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum GenerationStatus {
    #[default]
    Pending,
    Analyzing,
    Generating,
    Completed,
    Failed,
}

/// A feature extracted from requirements analysis
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExtractedFeature {
    pub name: String,
    pub description: String,
    pub layer: Option<String>,
    pub priority: Option<i32>,
}

/// Analysis result containing extracted features
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AnalysisResult {
    pub features: Vec<ExtractedFeature>,
    pub summary: Option<String>,
}

/// Project requirements with generation state
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ProjectRequirements {
    pub id: Uuid,
    pub project_id: Uuid,
    pub raw_requirements: String,
    pub prd_content: Option<String>,
    pub analysis_result: Option<String>, // JSON-serialized AnalysisResult
    pub generation_status: GenerationStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectRequirements {
    /// Parse the analysis_result JSON into an AnalysisResult struct
    pub fn parsed_analysis(&self) -> Option<AnalysisResult> {
        self.analysis_result
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
    }
}

/// Request body for creating requirements
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateProjectRequirements {
    pub raw_requirements: String,
    pub prd_content: Option<String>,
}

/// Response for requirements status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProjectRequirementsStatus {
    pub id: Uuid,
    pub project_id: Uuid,
    pub generation_status: GenerationStatus,
    pub analysis_result: Option<AnalysisResult>,
    pub tasks_generated: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectRequirements {
    pub async fn create(
        pool: &SqlitePool,
        id: Uuid,
        project_id: Uuid,
        data: &CreateProjectRequirements,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            ProjectRequirements,
            r#"
            INSERT INTO project_requirements (id, project_id, raw_requirements, prd_content)
            VALUES ($1, $2, $3, $4)
            RETURNING
                id              as "id!: Uuid",
                project_id      as "project_id!: Uuid",
                raw_requirements,
                prd_content,
                analysis_result,
                generation_status as "generation_status!: GenerationStatus",
                error_message,
                created_at      as "created_at!: DateTime<Utc>",
                updated_at      as "updated_at!: DateTime<Utc>"
            "#,
            id,
            project_id,
            data.raw_requirements,
            data.prd_content,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectRequirements,
            r#"
            SELECT
                id              as "id!: Uuid",
                project_id      as "project_id!: Uuid",
                raw_requirements,
                prd_content,
                analysis_result,
                generation_status as "generation_status!: GenerationStatus",
                error_message,
                created_at      as "created_at!: DateTime<Utc>",
                updated_at      as "updated_at!: DateTime<Utc>"
            FROM project_requirements
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectRequirements,
            r#"
            SELECT
                id              as "id!: Uuid",
                project_id      as "project_id!: Uuid",
                raw_requirements,
                prd_content,
                analysis_result,
                generation_status as "generation_status!: GenerationStatus",
                error_message,
                created_at      as "created_at!: DateTime<Utc>",
                updated_at      as "updated_at!: DateTime<Utc>"
            FROM project_requirements
            WHERE project_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: GenerationStatus,
        error_message: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE project_requirements
            SET generation_status = $2,
                error_message = $3,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            "#,
            id,
            status,
            error_message
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_analysis_result(
        pool: &SqlitePool,
        id: Uuid,
        analysis_result: &AnalysisResult,
    ) -> Result<(), sqlx::Error> {
        let json = serde_json::to_string(analysis_result)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        sqlx::query!(
            r#"
            UPDATE project_requirements
            SET analysis_result = $2,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            "#,
            id,
            json
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM project_requirements WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM project_requirements WHERE project_id = $1",
            project_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}
