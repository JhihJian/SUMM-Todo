use crate::cli::ListArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;
use crate::task::Status;
use crate::time_parse::parse_since;

pub fn execute(db: &Database, args: ListArgs, output: &Output) -> Result<String, TodoError> {
    let status = if args.status.is_empty() {
        // Default: show only active tasks (exclude done and cancelled)
        if args.all {
            None
        } else {
            Some(vec![Status::Pending, Status::InProgress, Status::Blocked])
        }
    } else {
        Some(
            args.status
                .iter()
                .map(|s| s.parse())
                .collect::<Result<Vec<Status>, _>>()?,
        )
    };

    let tags = if args.tag.is_empty() {
        None
    } else {
        Some(args.tag)
    };

    let priority = args.pri.map(|p| p.parse()).transpose()?;
    let creator = args.creator.map(|c| c.parse()).transpose()?;
    let since = args.since.map(|s| parse_since(&s)).transpose()?;

    let filter = TaskFilter {
        status,
        tags,
        priority,
        parent_id: args.parent,
        creator,
        since,
        limit: args.limit,
        sort: None,
        overdue: args.overdue,
        project_id: None,
    };

    let tasks = db.list_tasks(&filter)?;
    Ok(output.task_list(&tasks))
}
