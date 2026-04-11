use crate::cli::EditArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::Status;

pub fn execute(db: &Database, args: EditArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db
        .get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;

    // Terminal states cannot be edited
    if task.status == Status::Done || task.status == Status::Cancelled {
        return Err(TodoError::InvalidTransition {
            from: task.status.to_string(),
            to: "edit".to_string(),
        });
    }

    // Update fields
    if let Some(title) = args.title {
        task.title = title;
    }
    if let Some(pri) = args.priority {
        task.priority = pri.parse()?;
    }
    if let Some(due) = args.due {
        task.due = Some(crate::time_parse::parse_due(&due)?);
    }

    // Handle content/description
    if args.clear_content {
        task.content = None;
    } else if let Some(content) = args.description {
        task.content = Some(content);
    }

    // Handle tag additions/removals
    for tag in args.tag {
        if let Some(remove_tag) = tag.strip_prefix('-') {
            task.tags.retain(|t| t != remove_tag);
        } else if let Some(add_tag) = tag.strip_prefix('+') {
            if !task.tags.contains(&add_tag.to_string()) {
                task.tags.push(add_tag.to_string());
            }
        } else {
            // Treat as addition without + prefix
            if !task.tags.contains(&tag) {
                task.tags.push(tag);
            }
        }
    }

    db.update_task(&task)?;
    Ok(output.task(&task))
}
