CREATE TABLE tasks (
  id            TEXT PRIMARY KEY,
  title         TEXT NOT NULL,
  creator       TEXT NOT NULL DEFAULT 'human' CHECK(creator IN ('human', 'agent')),
  created_at    TEXT NOT NULL DEFAULT (datetime('now')),

  priority      TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('high', 'medium', 'low')),
  tags          TEXT DEFAULT '[]',
  parent_id     TEXT REFERENCES tasks(id),
  due           TEXT,

  status        TEXT NOT NULL DEFAULT 'pending'
                CHECK(status IN ('pending', 'in_progress', 'blocked', 'done', 'cancelled')),
  assignee      TEXT CHECK(assignee IS NULL OR assignee IN ('human', 'agent')),
  blocked_reason TEXT,

  result        TEXT,
  artifacts     TEXT DEFAULT '[]',
  log           TEXT,
  started_at    TEXT,
  finished_at   TEXT
);

CREATE INDEX idx_status ON tasks(status);
CREATE INDEX idx_priority ON tasks(priority);
CREATE INDEX idx_created ON tasks(created_at);
CREATE INDEX idx_parent ON tasks(parent_id);
