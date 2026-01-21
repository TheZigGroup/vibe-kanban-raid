-- Agent activity settings per project
CREATE TABLE IF NOT EXISTS project_agent_settings (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL UNIQUE REFERENCES projects(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 0,
    interval_seconds INTEGER NOT NULL DEFAULT 60,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Log of agent decisions for transparency
CREATE TABLE IF NOT EXISTS agent_activity_logs (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    action TEXT NOT NULL,  -- 'selected', 'skipped', 'error'
    reasoning TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient project lookups
CREATE INDEX IF NOT EXISTS idx_project_agent_settings_project_id ON project_agent_settings(project_id);
CREATE INDEX IF NOT EXISTS idx_agent_activity_logs_project_id ON agent_activity_logs(project_id);
CREATE INDEX IF NOT EXISTS idx_agent_activity_logs_created_at ON agent_activity_logs(created_at DESC);
