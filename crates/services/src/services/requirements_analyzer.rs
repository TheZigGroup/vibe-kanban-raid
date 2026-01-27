//! Service for analyzing requirements and generating tasks using Claude AI.

use db::models::{
    project_requirements::{
        AnalysisResult, CreateProjectRequirements, ExtractedFeature, GenerationStatus,
        ProjectRequirements,
    },
    task::{CreateTask, Task, TaskLayer, TaskType},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use tracing::{error, info};
use uuid::Uuid;

use super::claude_api::{ClaudeApiClient, ClaudeApiError};
use super::codebase_rules;

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
    task_type: Option<String>,
    testing_criteria: Option<String>,
    files_to_modify: Option<Vec<String>>,
    post_task_actions: Option<String>,
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

IMPORTANT: This is for an EXISTING working project. Features should be analyzed in the context of extending/modifying the existing codebase.

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
   - "data": Database models, schemas, migrations (SQLite)
   - "backend": API endpoints, business logic, services (Node.js)
   - "frontend": UI components, pages, user interactions (React + Vite + Zustand)
   - "fullstack": Features spanning multiple layers
   - "devops": Infrastructure, deployment, CI/CD
   - "testing": Test coverage, test utilities
3. Assign a priority (1=highest, 5=lowest) based on dependencies and importance
4. Consider cross-layer dependencies - features that require data models, API contracts, and UI components

## Output Format
Return ONLY valid JSON with this structure:
```json
{
  "features": [
    {
      "name": "Feature name",
      "description": "Brief description of what needs to be built, including any cross-layer dependencies",
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
            "You are a software architect analyzing requirements for an EXISTING project to extract features. \
             Consider that you're working with an established codebase and architecture. Be concise and practical. \
             Focus on actionable features that extend or modify the existing system. Output valid JSON only."
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

    /// Phase 2: Generate implementation tasks from features using mock-first, architecture-first approach
    async fn generate_tasks_from_features(
        &self,
        project_id: Uuid,
        features: &[ExtractedFeature],
    ) -> Result<usize, RequirementsAnalyzerError> {
        // Generate all tasks at once using the architecture-first approach
        let tasks = self.generate_architecture_first_tasks(features).await?;

        let mut total_tasks = 0;
        for task in tasks {
            let layer = task.layer.and_then(|l| parse_layer(&l));
            let task_type = task.task_type.and_then(|t| parse_task_type(&t));

            // Determine sequence based on task type
            let sequence = calculate_sequence(&task_type, total_tasks);

            let create_task = CreateTask::ai_generated(
                project_id,
                task.title,
                Some(task.description),
                layer,
                task_type,
                sequence,
                task.testing_criteria,
                task.post_task_actions,
            );

            Task::create(&self.pool, &create_task, Uuid::new_v4()).await?;
            total_tasks += 1;
        }

        Ok(total_tasks)
    }

    /// Generate tasks using mock-first, architecture-first approach
    async fn generate_architecture_first_tasks(
        &self,
        features: &[ExtractedFeature],
    ) -> Result<Vec<GeneratedTask>, RequirementsAnalyzerError> {
        let features_json = features
            .iter()
            .map(|f| {
                format!(
                    r#"  - Name: {}
    Description: {}
    Layer: {}"#,
                    f.name,
                    f.description,
                    f.layer.as_deref().unwrap_or("fullstack")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let rules = codebase_rules::get_all_rules();

        let prompt = format!(
            r#"Generate implementation tasks for the following features.

IMPORTANT: This is an EXISTING working project. You must analyze the existing codebase structure and generate tasks that work with the existing files and architecture.

## ARCHITECTURE RULES (MUST FOLLOW)
{}

## Features to Implement
{}

## Task Generation Strategy

Generate tasks in this EXACT ORDER:

### 1. Architecture Tasks (task_type: "architecture")
Analyze existing architecture and define any new models/contracts needed:
- Identify existing data models, API patterns, state management
- Define any NEW interfaces/types needed for the features
- Design database schema changes if needed
- Create database migrations if database changes are required
- Ensure database is initialized before proceeding

### 2. Implementation Tasks (task_type: "implementation")
Build real implementations working with the existing codebase:
- Modify existing files or create new ones following project conventions
- Integrate with existing patterns and services
- Reference specific files that need modification
- Implement full functionality (no mocks - build the real thing)
- Include database operations if needed (queries, mutations)
- Run and test migrations as part of implementation

### 3. Integration Task (task_type: "integration")
Wire new features into the existing application:
- Connect new endpoints/components to existing infrastructure
- Verify database migrations have been applied
- End-to-end testing of the complete flow
- Ensure all database tables and schemas are correct

## Output Format
Return ONLY valid JSON:
```json
{{
  "tasks": [
    {{
      "title": "Short task title",
      "description": "Detailed description explaining WHAT to build and HOW it integrates with existing code",
      "layer": "backend|frontend|data|fullstack|devops|testing",
      "task_type": "architecture|implementation|integration",
      "testing_criteria": "Specific, verifiable criteria to confirm this task is complete",
      "files_to_modify": ["path/to/file1.ts", "path/to/file2.tsx"],
      "post_task_actions": "<markdown template - see below>"
    }}
  ]
}}
```

## post_task_actions Format
MUST be markdown text that will be appended to .progress file. Include:
- Task title as heading
- Status with timestamp
- Summary of what was done
- List of files changed
- Testing results
- Separator line

IMPORTANT:
- Analyze the EXISTING project structure before generating tasks
- Reference specific existing files that need modification in files_to_modify
- FOLLOW THE ARCHITECTURE RULES STRICTLY - do not recreate components that already exist
- For frontend tasks: DO NOT create new navbar/sidebar, use existing layout components
- For backend tasks: Follow the routing, service, and model patterns
- For database tasks: ALWAYS create migration files, run pnpm run prepare-db, never use npm run init-db
- Database must exist before migrations run - include database creation check in architecture tasks
- post_task_actions should be actual markdown text, not a template with placeholders
- This markdown will be appended to .progress file when task completes
- Include a "Rules Followed" section in post_task_actions listing which architecture rules were applied
- Include a "Database Changes" section if migrations were created
"#,
            rules,
            features_json
        );

        let system = Some(
            "You are a software architect analyzing an EXISTING codebase and generating implementation tasks. \
             You must analyze the existing project structure, identify patterns, and generate tasks that work with \
             the existing architecture. Each task should reference specific files to modify and include a markdown \
             template for progress tracking. Use a mock-first approach where appropriate. Output valid JSON only."
                .to_string(),
        );

        let response: TaskGenerationResponse = self.claude.ask_json_with_max_tokens(&prompt, system, 8192).await?;
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

fn parse_task_type(s: &str) -> Option<TaskType> {
    match s.to_lowercase().as_str() {
        "architecture" => Some(TaskType::Architecture),
        "implementation" => Some(TaskType::Implementation),
        "integration" => Some(TaskType::Integration),
        _ => None,
    }
}

/// Calculate sequence number based on task type to ensure proper ordering.
/// Sequence ranges:
/// - Architecture: 0-99
/// - Implementation: 100-899
/// - Integration: 900+
fn calculate_sequence(task_type: &Option<TaskType>, task_index: usize) -> i32 {
    let base = match task_type {
        Some(TaskType::Architecture) => 0,
        Some(TaskType::Implementation) => 100,
        Some(TaskType::Integration) => 900,
        None => 100, // Default to implementation range
    };
    base + (task_index as i32 % 100)
}
