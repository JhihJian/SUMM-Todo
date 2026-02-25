use crate::cli::AddArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::id::generate_id;
use crate::output::Output;
use crate::task::{Creator, Priority, Task};
use crate::time_parse::parse_due;

pub fn execute(db: &Database, args: AddArgs, output: &Output) -> Result<String, TodoError> {
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

    let mut task = Task::new(id, args.title);
    task.creator = creator;
    task.priority = priority;
    task.tags = args.tag;
    task.parent_id = args.parent;
    task.due = due;
    task.content = args.description;

    db.insert_task(&task)?;
    Ok(output.task(&task))
}
