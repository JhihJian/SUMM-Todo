use crate::cli::ShowArgs;
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;

pub fn execute(db: &Database, args: ShowArgs, output: &Output) -> Result<String, TodoError> {
    let task = db
        .get_task(&args.id)?
        .ok_or_else(|| TodoError::TaskNotFound(args.id.clone()))?;
    Ok(output.task(&task))
}
