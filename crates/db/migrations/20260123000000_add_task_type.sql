-- Add task_type column for categorizing tasks in the mock-first architecture-first approach
-- Types: architecture (schemas/contracts), mock (mock implementations), implementation (real code), integration (wire layers)
ALTER TABLE tasks ADD COLUMN task_type TEXT DEFAULT 'implementation'
    CHECK (task_type IS NULL OR task_type IN ('architecture', 'mock', 'implementation', 'integration'));
