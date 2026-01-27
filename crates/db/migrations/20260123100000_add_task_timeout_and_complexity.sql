-- Track when task entered current stage (separate from updated_at for accurate timeout detection)
ALTER TABLE tasks ADD COLUMN stage_started_at TEXT DEFAULT NULL;

-- Track complexity score from AI analysis (1-10 scale)
ALTER TABLE tasks ADD COLUMN complexity_score INTEGER DEFAULT NULL;

-- Link subtasks to parent task when complex tasks are broken down
ALTER TABLE tasks ADD COLUMN parent_task_id BLOB DEFAULT NULL REFERENCES tasks(id) ON DELETE SET NULL;

-- Initialize stage_started_at for currently active tasks
UPDATE tasks SET stage_started_at = updated_at WHERE status IN ('inprogress', 'inreview');

-- Indexes for efficient timeout and subtask queries
CREATE INDEX IF NOT EXISTS idx_tasks_stage_timeout ON tasks(status, stage_started_at)
  WHERE status IN ('inprogress', 'inreview') AND stage_started_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_parent_task_id ON tasks(parent_task_id)
  WHERE parent_task_id IS NOT NULL;

-- Add recursive breakdown depth setting to project_agent_settings
ALTER TABLE project_agent_settings ADD COLUMN max_breakdown_depth INTEGER NOT NULL DEFAULT 1;
