use crate::cli::LogArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;
use crate::task::Status;
use crate::time_parse::parse_since;

pub fn execute(db: &Database, args: LogArgs, output: &Output) -> Result<String, TodoError> {
    let since = if args.today {
        Some(parse_since("today")?)
    } else {
        args.since.map(|s| parse_since(&s)).transpose()?
    };

    let tags = args.tag.map(|t| vec![t]);

    let filter = TaskFilter {
        status: Some(vec![Status::Done]),
        tags,
        since,
        limit: Some(100),
        ..Default::default()
    };

    let tasks = db.list_tasks(&filter)?;
    Ok(output.log(&tasks))
}
