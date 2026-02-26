use crate::cli::{ProjectAddArgs, ProjectCommand, ProjectDeleteArgs, ProjectEditArgs, ProjectListArgs, ProjectShowArgs};
use crate::db::Database;
use crate::error::TodoError;
use crate::id::generate_id;
use crate::output::Output;
use crate::task::Project;

pub fn execute(db: &Database, command: ProjectCommand, output: &Output) -> Result<String, TodoError> {
    match command {
        ProjectCommand::ProjectAdd(args) => add(db, args, output),
        ProjectCommand::ProjectEdit(args) => edit(db, args, output),
        ProjectCommand::ProjectList(args) => list(db, args, output),
        ProjectCommand::ProjectShow(args) => show(db, args, output),
        ProjectCommand::ProjectDelete(args) => delete(db, args, output),
    }
}

fn add(db: &Database, args: ProjectAddArgs, output: &Output) -> Result<String, TodoError> {
    // Check if project already exists
    if db.get_project_by_name(&args.name)?.is_some() {
        return Err(TodoError::ProjectExists(args.name));
    }

    let id = generate_id(&db.conn)?;
    let mut project = Project::new(id, args.name);
    project.description = args.description;

    db.insert_project(&project)?;
    Ok(output.project(&project))
}

fn edit(db: &Database, args: ProjectEditArgs, output: &Output) -> Result<String, TodoError> {
    let project = db
        .get_project_by_name(&args.name)?
        .ok_or_else(|| TodoError::ProjectNotFound(args.name.clone()))?;

    let mut updated = project.clone();

    if let Some(new_name) = args.new_name {
        // Check if new name already exists
        if new_name != project.name {
            if db.get_project_by_name(&new_name)?.is_some() {
                return Err(TodoError::ProjectExists(new_name));
            }
        }
        updated.name = new_name;
    }

    if let Some(description) = args.description {
        updated.description = Some(description);
    }

    db.update_project(&updated)?;
    Ok(output.project(&updated))
}

fn list(db: &Database, _args: ProjectListArgs, output: &Output) -> Result<String, TodoError> {
    let projects = db.list_projects()?;
    let mut result = String::new();

    for project in projects {
        let stats = db.get_project_stats(&project.id)?;
        result.push_str(&output.project_list_item(&project, &stats));
        result.push_str("\n\n");
    }

    if result.is_empty() {
        result = "No projects found.\n".to_string();
    }

    Ok(result.trim_end().to_string())
}

fn show(db: &Database, args: ProjectShowArgs, output: &Output) -> Result<String, TodoError> {
    let project = db
        .get_project_by_name(&args.name)?
        .ok_or_else(|| TodoError::ProjectNotFound(args.name.clone()))?;

    let stats = db.get_project_stats(&project.id)?;
    let recent_tasks = db.get_project_recent_tasks(&project.id, args.limit)?;

    Ok(output.project_detail(&project, &stats, &recent_tasks))
}

fn delete(db: &Database, args: ProjectDeleteArgs, output: &Output) -> Result<String, TodoError> {
    let project = db
        .get_project_by_name(&args.name)?
        .ok_or_else(|| TodoError::ProjectNotFound(args.name.clone()))?;

    db.delete_project(&project.id)?;

    Ok(output.project_deleted(&project))
}
