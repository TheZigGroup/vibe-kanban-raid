//! Service for analyzing requirements and generating tasks using Claude AI.

use db::models::{
    project_requirements::{
        AnalysisResult, CreateProjectRequirements, ExtractedFeature, GenerationStatus,
        ProjectRequirements,
    },
    task::{CreateTask, Task, TaskLayer},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use tracing::{error, info};
use uuid::Uuid;

use super::claude_api::{ClaudeApiClient, ClaudeApiError};

#[derive(Debug, Error)]
pub enum RequirementsAnalyzerError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("claude api error: {0}")]
    ClaudeApi(#[from] ClaudeApiError),
    #[error("requirements not found")]
    NotFound,
    #[error("analysis already in progress")]
    AlreadyInProgress,
}

/// Response from feature extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FeatureExtractionResponse {
    features: Vec<ExtractedFeatureResponse>,
    summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtractedFeatureResponse {
    name: String,
    description: String,
    layer: Option<String>,
    priority: Option<i32>,
}

/// Response from task generation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskGenerationResponse {
    tasks: Vec<GeneratedTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeneratedTask {
    title: String,
    description: String,
    layer: Option<String>,
}

/// Service for analyzing requirements and generating tasks
pub struct RequirementsAnalyzer {
    pool: SqlitePool,
    claude: ClaudeApiClient,
}

impl RequirementsAnalyzer {
    pub fn new(pool: SqlitePool) -> Result<Self, RequirementsAnalyzerError> {
        let claude = ClaudeApiClient::from_env()?;
        Ok(Self { pool, claude })
    }

    pub fn with_client(pool: SqlitePool, claude: ClaudeApiClient) -> Self {
        Self { pool, claude }
    }

    /// Create a new requirements record and start async analysis
    pub async fn create_and_analyze(
        &self,
        project_id: Uuid,
        data: CreateProjectRequirements,
    ) -> Result<ProjectRequirements, RequirementsAnalyzerError> {
        let id = Uuid::new_v4();
        let requirements =
            ProjectRequirements::create(&self.pool, id, project_id, &data).await?;

        info!(
            requirements_id = %id,
            project_id = %project_id,
            "Created requirements record, starting analysis"
        );

        // Clone what we need for the spawned task
        let pool = self.pool.clone();
        let claude = self.claude.clone();
        let raw_requirements = data.raw_requirements.clone();
        let prd_content = data.prd_content.clone();

        // Spawn the analysis in the background
        tokio::spawn(async move {
            let analyzer = RequirementsAnalyzer::with_client(pool, claude);
            if let Err(e) = analyzer
                .run_analysis(id, project_id, &raw_requirements, prd_content.as_deref())
                .await
            {
                error!(error = %e, "Requirements analysis failed");
            }
        });

        Ok(requirements)
    }

    /// Run the full analysis and task generation pipeline
    async fn run_analysis(
        &self,
        requirements_id: Uuid,
        project_id: Uuid,
        raw_requirements: &str,
        prd_content: Option<&str>,
    ) -> Result<(), RequirementsAnalyzerError> {
        // Phase 1: Analyze requirements to extract features
        ProjectRequirements::update_status(
            &self.pool,
            requirements_id,
            GenerationStatus::Analyzing,
            None,
        )
        .await?;

        let analysis_result = match self.analyze_requirements(raw_requirements, prd_content).await {
            Ok(result) => result,
            Err(e) => {
                ProjectRequirements::update_status(
                    &self.pool,
                    requirements_id,
                    GenerationStatus::Failed,
                    Some(&e.to_string()),
                )
                .await?;
                return Err(e);
            }
        };

        // Save analysis result
        ProjectRequirements::update_analysis_result(&self.pool, requirements_id, &analysis_result)
            .await?;

        info!(
            requirements_id = %requirements_id,
            feature_count = analysis_result.features.len(),
            "Completed feature extraction"
        );

        // Phase 2: Generate tasks from features
        ProjectRequirements::update_status(
            &self.pool,
            requirements_id,
            GenerationStatus::Generating,
            None,
        )
        .await?;

        match self
            .generate_tasks_from_features(project_id, &analysis_result.features)
            .await
        {
            Ok(task_count) => {
                info!(
                    requirements_id = %requirements_id,
                    task_count = task_count,
                    "Task generation completed"
                );
                ProjectRequirements::update_status(
                    &self.pool,
                    requirements_id,
                    GenerationStatus::Completed,
                    None,
                )
                .await?;
            }
            Err(e) => {
                ProjectRequirements::update_status(
                    &self.pool,
                    requirements_id,
                    GenerationStatus::Failed,
                    Some(&e.to_string()),
                )
                .await?;
                return Err(e);
            }
        }

        Ok(())
    }

    /// Phase 1: Analyze requirements and extract features
    async fn analyze_requirements(
        &self,
        raw_requirements: &str,
        prd_content: Option<&str>,
    ) -> Result<AnalysisResult, RequirementsAnalyzerError> {
        let mut prompt = format!(
            r#"Analyze the following project requirements and extract distinct features that need to be implemented.

## Requirements
{}
"#,
            raw_requirements
        );

        if let Some(prd) = prd_content {
            prompt.push_str(&format!(
                r#"
## Additional PRD Content
{}
"#,
                prd
            ));
        }

        prompt.push_str(
            r#"
## Instructions
1. Identify distinct features that need to be implemented
2. For each feature, determine which layer it primarily belongs to:
   - "data": Database models, schemas, migrations
   - "backend": API endpoints, business logic, services
   - "frontend": UI components, pages, user interactions
   - "fullstack": Features spanning multiple layers
   - "devops": Infrastructure, deployment, CI/CD
   - "testing": Test coverage, test utilities
3. Assign a priority (1=highest, 5=lowest) based on dependencies and importance

## Output Format
Return ONLY valid JSON with this structure:
```json
{
  "features": [
    {
      "name": "Feature name",
      "description": "Brief description of what needs to be built",
      "layer": "backend|frontend|data|fullstack|devops|testing",
      "priority": 1
    }
  ],
  "summary": "Brief summary of the overall project scope"
}
```
"#,
        );

        let system = Some(
            "You are a software architect analyzing requirements to extract features for a kanban board. \
             Be concise and practical. Focus on actionable features that can be implemented as tasks. \
             Output valid JSON only."
                .to_string(),
        );

        let response: FeatureExtractionResponse = self.claude.ask_json(&prompt, system).await?;

        Ok(AnalysisResult {
            features: response
                .features
                .into_iter()
                .map(|f| ExtractedFeature {
                    name: f.name,
                    description: f.description,
                    layer: f.layer,
                    priority: f.priority,
                })
                .collect(),
            summary: response.summary,
        })
    }

    /// Phase 2: Generate implementation tasks from features
    async fn generate_tasks_from_features(
        &self,
        project_id: Uuid,
        features: &[ExtractedFeature],
    ) -> Result<usize, RequirementsAnalyzerError> {
        let mut total_tasks = 0;
        let mut sequence = 0;

        // Sort features by priority
        let mut sorted_features: Vec<_> = features.iter().collect();
        sorted_features.sort_by_key(|f| f.priority.unwrap_or(3));

        for feature in sorted_features {
            let tasks = self.generate_tasks_for_feature(feature).await?;

            for task in tasks {
                let layer = task.layer.and_then(|l| parse_layer(&l));

                let create_task = CreateTask::ai_generated(
                    project_id,
                    task.title,
                    Some(task.description),
                    layer,
                    sequence,
                );

                Task::create(&self.pool, &create_task, Uuid::new_v4()).await?;
                sequence += 1;
                total_tasks += 1;
            }
        }

        Ok(total_tasks)
    }

    /// Generate implementation tasks for a single feature
    async fn generate_tasks_for_feature(
        &self,
        feature: &ExtractedFeature,
    ) -> Result<Vec<GeneratedTask>, RequirementsAnalyzerError> {
        let prompt = format!(
            r#"Generate implementation tasks for the following feature:

## Feature
Name: {}
Description: {}
Layer: {}

## Instructions
Break this feature down into specific, actionable implementation tasks.
Each task should be:
- Small enough to complete in one focused session
- Clear about what needs to be done
- Assigned to the appropriate layer (data/backend/frontend/fullstack/devops/testing)

## Output Format
Return ONLY valid JSON:
```json
{{
  "tasks": [
    {{
      "title": "Short task title",
      "description": "Detailed description of what to implement",
      "layer": "backend|frontend|data|fullstack|devops|testing"
    }}
  ]
}}
```
"#,
            feature.name,
            feature.description,
            feature.layer.as_deref().unwrap_or("fullstack")
        );

        let system = Some(
            "You are a software engineer breaking down features into implementation tasks. \
             Create practical, actionable tasks. Output valid JSON only."
                .to_string(),
        );

        let response: TaskGenerationResponse = self.claude.ask_json(&prompt, system).await?;
        Ok(response.tasks)
    }

    /// Get the current status of requirements analysis
    pub async fn get_status(
        &self,
        project_id: Uuid,
    ) -> Result<Option<ProjectRequirements>, RequirementsAnalyzerError> {
        Ok(ProjectRequirements::find_by_project_id(&self.pool, project_id).await?)
    }

    /// Delete requirements and optionally the generated tasks
    pub async fn delete(
        &self,
        project_id: Uuid,
        delete_tasks: bool,
    ) -> Result<(), RequirementsAnalyzerError> {
        if delete_tasks {
            // Delete AI-generated tasks for this project
            sqlx::query("DELETE FROM tasks WHERE project_id = $1 AND source = 'ai_generated'")
                .bind(project_id)
                .execute(&self.pool)
                .await?;
        }

        ProjectRequirements::delete_by_project_id(&self.pool, project_id).await?;
        Ok(())
    }
}

fn parse_layer(s: &str) -> Option<TaskLayer> {
    match s.to_lowercase().as_str() {
        "data" => Some(TaskLayer::Data),
        "backend" => Some(TaskLayer::Backend),
        "frontend" => Some(TaskLayer::Frontend),
        "fullstack" => Some(TaskLayer::Fullstack),
        "devops" => Some(TaskLayer::Devops),
        "testing" => Some(TaskLayer::Testing),
        _ => None,
    }
}
