-- Add requirements generation support

-- Store requirements and generation state
CREATE TABLE IF NOT EXISTS project_requirements (
    id BLOB PRIMARY KEY NOT NULL,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    raw_requirements TEXT NOT NULL,
    prd_content TEXT,
    analysis_result TEXT,  -- JSON: extracted features
    generation_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (generation_status IN ('pending', 'analyzing', 'generating', 'completed', 'failed')),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Index for quick lookup by project
CREATE INDEX idx_project_requirements_project_id ON project_requirements(project_id);

-- Track auto-generated tasks
ALTER TABLE tasks ADD COLUMN source TEXT DEFAULT 'manual'
    CHECK (source IN ('manual', 'ai_generated'));
ALTER TABLE tasks ADD COLUMN layer TEXT
    CHECK (layer IS NULL OR layer IN ('data', 'backend', 'frontend', 'fullstack', 'devops', 'testing'));
ALTER TABLE tasks ADD COLUMN sequence INTEGER;

-- Index for ordering generated tasks
CREATE INDEX idx_tasks_sequence ON tasks(project_id, sequence) WHERE sequence IS NOT NULL;
