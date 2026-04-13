use std::io::Read;

use crate::cli::ImportArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::id::generate_id;
use crate::output::Output;
use crate::task::Task;

pub fn execute(db: &Database, args: ImportArgs, _output: &Output) -> Result<String, TodoError> {
    let json_str = if args.file == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(TodoError::Io)?;
        buf
    } else {
        std::fs::read_to_string(&args.file).map_err(TodoError::Io)?
    };

    let items: Vec<serde_json::Value> = serde_json::from_str(&json_str)
        .map_err(|e| TodoError::ParseError(e.to_string()))?;

    let mut count = 0;
    for item in &items {
        let title = item["title"].as_str().ok_or_else(|| {
            TodoError::InvalidInput("Each item must have a 'title' field".into())
        })?;

        let id = generate_id(&db.conn)?;
        let mut task = Task::new(id, title.to_string());

        if let Some(pri) = item["priority"].as_str() {
            task.priority = pri.parse()?;
        }
        if let Some(tags) = item["tags"].as_array() {
            task.tags = tags
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(creator) = item["creator"].as_str() {
            task.creator = creator.parse()?;
        }
        if let Some(parent) = item["parent_id"].as_str() {
            task.parent_id = Some(parent.to_string());
        }

        db.insert_task(&task)?;
        count += 1;
    }

    Ok(serde_json::json!({"imported": count}).to_string())
}
