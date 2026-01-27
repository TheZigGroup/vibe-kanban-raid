-- Add flag to prevent automatic task breakdown
ALTER TABLE tasks ADD COLUMN prevent_breakdown INTEGER NOT NULL DEFAULT 0;
