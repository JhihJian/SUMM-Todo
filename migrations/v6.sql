-- v6: Add pre_id column for task dependency / ordering
ALTER TABLE tasks ADD COLUMN pre_id TEXT REFERENCES tasks(id);

CREATE INDEX IF NOT EXISTS idx_pre ON tasks(pre_id);
