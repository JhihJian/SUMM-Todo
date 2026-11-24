use crate::cli::StartArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Creator, Status, TransitionContext};

pub fn execute(db: &Database, args: StartArgs, output: &Output) -> Result<String, TodoError> {
    let mut task = db
        .get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;

    // Reject starting a task whose dependency is not done yet
    if let Some(ref pre_id) = task.pre_id {
        let pre = db
            .get_task(pre_id)?
            .ok_or_else(|| TodoError::TaskNotFound(pre_id.clone()))?;
        if pre.status != Status::Done {
            return Err(TodoError::DependencyNotSatisfied(pre_id.clone()));
        }
    }

    let assignee: Creator = args
        .assignee
        .map(|a| a.parse())
        .transpose()?
        .unwrap_or(Creator::Human);

    task.transition(
        Status::InProgress,
        TransitionContext {
            assignee: Some(assignee),
            result: None,
            artifacts: None,
            log: None,
            blocked_reason: None,
        },
    )?;

    db.update_task(&task)?;
    Ok(output.task(&task))
}
