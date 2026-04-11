-- v5: Add updated_at columns, sync_config, sync_log, triggers

-- Add updated_at column to tasks table
ALTER TABLE tasks ADD COLUMN updated_at TEXT;
UPDATE tasks SET updated_at = created_at WHERE updated_at IS NULL;

-- Add updated_at column to projects table
ALTER TABLE projects ADD COLUMN updated_at TEXT;
UPDATE projects SET updated_at = created_at WHERE updated_at IS NULL;

-- Sync configuration
CREATE TABLE IF NOT EXISTS sync_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Change tracking log (autoincrement id avoids PK conflicts on same-timestamp entries)
CREATE TABLE IF NOT EXISTS sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    recorded_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sync_log_entity ON sync_log(entity_type, entity_id);

-- Triggers for tasks
CREATE TRIGGER IF NOT EXISTS sync_track_task_insert
AFTER INSERT ON tasks
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('task', NEW.id, 'upsert', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
END;

CREATE TRIGGER IF NOT EXISTS sync_track_task_update
AFTER UPDATE ON tasks
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('task', NEW.id, 'upsert', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
END;

CREATE TRIGGER IF NOT EXISTS sync_track_task_delete
AFTER DELETE ON tasks
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('task', OLD.id, 'delete', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
END;

-- Triggers for projects
CREATE TRIGGER IF NOT EXISTS sync_track_project_insert
AFTER INSERT ON projects
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('project', NEW.id, 'upsert', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
END;

CREATE TRIGGER IF NOT EXISTS sync_track_project_update
AFTER UPDATE ON projects
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('project', NEW.id, 'upsert', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
END;

CREATE TRIGGER IF NOT EXISTS sync_track_project_delete
AFTER DELETE ON projects
BEGIN
    INSERT INTO sync_log (entity_type, entity_id, action, recorded_at)
    VALUES ('project', OLD.id, 'delete', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
END;
