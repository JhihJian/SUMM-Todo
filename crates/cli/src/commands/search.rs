use crate::cli::SearchArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;

pub fn execute(db: &Database, args: SearchArgs, output: &Output) -> Result<String, TodoError> {
    let status = args.status.map(|s| s.parse()).transpose()?;

    let mut filter = TaskFilter {
        status: status.map(|s| vec![s]),
        ..Default::default()
    };

    if let Some(tag) = &args.tag {
        filter.tags = Some(vec![tag.clone()]);
    }

    let tasks = db.search_tasks(&args.query, args.regex, &filter)?;
    Ok(output.task_list(&tasks))
}
