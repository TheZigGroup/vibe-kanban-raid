-- Add post_task_actions field to tasks table
-- This field stores instructions for updating the .progress file when a task is completed
ALTER TABLE tasks ADD COLUMN post_task_actions TEXT;
