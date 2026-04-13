use crate::cli::NextArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Creator, Status, TransitionContext};

pub fn execute(db: &Database, args: NextArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db
        .get_next_task(args.tag.as_deref(), args.pri.as_deref())?
        .ok_or(TodoError::QueueEmpty)?;

    task.transition(
        Status::InProgress,
        TransitionContext {
            assignee: Some(Creator::Agent),
            result: None,
            artifacts: None,
            log: None,
            blocked_reason: None,
        },
    )?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
