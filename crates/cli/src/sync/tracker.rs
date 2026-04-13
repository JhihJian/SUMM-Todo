use crate::db::Database;
use crate::error::TodoError;
use crate::task::{Project, Task};

/// Queries sync_log for pending changes and resolves latest action per entity.
pub struct SyncTracker<'a> {
    db: &'a Database,
}

impl<'a> SyncTracker<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get pending changes from sync_log.
    /// Returns (upserted_tasks, deleted_task_ids, upserted_projects, deleted_project_ids).
    pub fn get_pending(&self) -> Result<(Vec<Task>, Vec<String>, Vec<Project>, Vec<String>), TodoError> {
        self.db.get_sync_pending()
    }

    /// Clear sync_log after successful push.
    pub fn clear(&self) -> Result<(), TodoError> {
        self.db.clear_sync_log()
    }
}
