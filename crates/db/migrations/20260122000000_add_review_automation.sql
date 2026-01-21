-- Review automation settings per project
CREATE TABLE IF NOT EXISTS project_review_settings (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL UNIQUE REFERENCES projects(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 0,
    auto_merge_enabled INTEGER NOT NULL DEFAULT 1,
    run_tests_enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_project_review_settings_project_id ON project_review_settings(project_id);
CREATE INDEX IF NOT EXISTS idx_project_review_settings_enabled ON project_review_settings(enabled) WHERE enabled = 1;

-- Review automation logs
CREATE TABLE IF NOT EXISTS review_automation_logs (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    action TEXT NOT NULL, -- 'test_passed', 'test_failed', 'merge_completed', 'merge_conflict', 'skipped', 'error'
    output TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_review_automation_logs_task ON review_automation_logs(task_id);
CREATE INDEX IF NOT EXISTS idx_review_automation_logs_workspace ON review_automation_logs(workspace_id);
CREATE INDEX IF NOT EXISTS idx_review_automation_logs_created_at ON review_automation_logs(created_at DESC);
