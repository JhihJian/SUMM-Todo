use crate::cli::CancelArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Status, TransitionContext};

pub fn execute(db: &Database, args: CancelArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db
        .get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;

    task.transition(
        Status::Cancelled,
        TransitionContext {
            assignee: None,
            result: None,
            artifacts: None,
            log: None,
            blocked_reason: None,
        },
    )?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
