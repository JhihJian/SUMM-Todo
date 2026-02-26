-- v3: Add projects table and project_id to tasks
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT NOT NULL
);

ALTER TABLE tasks ADD COLUMN project_id TEXT REFERENCES projects(id);

CREATE INDEX idx_tasks_project_id ON tasks(project_id);
CREATE INDEX idx_projects_name ON projects(name);
