use std::io::Write;

use crate::cli::ExportArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;

pub fn execute(db: &Database, args: ExportArgs, _output: &Output) -> Result<String, TodoError> {
    let mut filter = TaskFilter::default();
    filter.limit = None; // Export all tasks

    if !args.status.is_empty() {
        filter.status = Some(args.status.iter().filter_map(|s| s.parse().ok()).collect());
    }
    if !args.tag.is_empty() {
        filter.tags = Some(args.tag.clone());
    }

    let tasks = db.list_tasks(&filter)?;
    let json = serde_json::to_string_pretty(&tasks)
        .map_err(|e| TodoError::ParseError(e.to_string()))?;

    if let Some(path) = &args.file {
        let mut file = std::fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(format!("Exported {} tasks to {}", tasks.len(), path))
    } else {
        Ok(json)
    }
}
