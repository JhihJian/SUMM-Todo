use crate::cli::AddArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::id::generate_id;
use crate::output::Output;
use crate::task::{Creator, Priority, Task};
use crate::time_parse::parse_due;

pub fn execute(db: &Database, args: AddArgs, output: &Output) -> Result<String, TodoError> {
    // Parse project prefix: "project_name: task title"
    let (project_name, title) = parse_project_prefix(&args.title);

    let project = project_name
        .map(|name| {
            db.get_project_by_name(name)
                .and_then(|p| p.ok_or_else(|| TodoError::ProjectNotFound(name.to_string())))
        })
        .transpose()?;

    let priority: Priority = args
        .pri
        .map(|p| p.parse())
        .transpose()?
        .unwrap_or(Priority::Medium);

    let creator: Creator = args
        .creator
        .map(|c| c.parse())
        .transpose()?
        .unwrap_or(Creator::Human);

    let due = args.due.map(|d| parse_due(&d)).transpose()?;

    let id = generate_id(&db.conn)?;

    let mut task = Task::new(id, title);
    task.creator = creator;
    task.priority = priority;
    task.tags = args.tag;
    task.parent_id = args.parent;
    task.due = due;
    task.content = args.description;
    task.project_id = project.map(|p| p.id);

    db.insert_task(&task)?;
    Ok(output.task(&task))
}

/// Parse "project: title" format, returns (project_name, title).
/// Returns (None, original_title) if no project prefix.
fn parse_project_prefix(input: &str) -> (Option<&str>, String) {
    // Find the first colon that's followed by a space or is at the end
    if let Some(pos) = input.find(':') {
        let project = input[..pos].trim();
        let title = input[pos + 1..].trim();

        // Only treat as project prefix if project name is not empty
        if !project.is_empty() && !title.is_empty() {
            return (Some(project), title.to_string());
        }
    }
    (None, input.to_string())
}
