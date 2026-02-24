use crate::cli::DoneArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Status, TransitionContext};

pub fn execute(db: &Database, args: DoneArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db
        .get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;

    task.transition(
        Status::Done,
        TransitionContext {
            assignee: None,
            result: Some(args.result),
            artifacts: Some(args.artifact),
            log: args.log,
            blocked_reason: None,
        },
    )?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
