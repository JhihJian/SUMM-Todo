use assert_cmd::Command;
use tempfile::tempdir;

fn todo_cmd() -> Command {
    Command::cargo_bin("todo").unwrap()
}

#[test]
fn project_add_list_show_delete() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Add project
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "add", "MyProject", "-d", "Test description"])
        .assert()
        .success();

    // List projects
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "list"])
        .assert()
        .stdout(predicates::str::contains("MyProject"))
        .success();

    // Show project
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "show", "MyProject"])
        .assert()
        .stdout(predicates::str::contains("MyProject"))
        .stdout(predicates::str::contains("Test description"))
        .success();

    // Delete project
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "delete", "MyProject"])
        .assert()
        .success();
}

#[test]
fn add_task_with_project() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Create project first
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "add", "Work"])
        .assert()
        .success();

    // Add task with project prefix
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["add", "Work: Do something"])
        .assert()
        .success();

    // List should show task (in TOON format, project_id is shown as UUID)
    // Use -p (pretty) to see the project name grouping
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["-p", "list"])
        .assert()
        .stdout(predicates::str::contains("Work"))
        .stdout(predicates::str::contains("Do something"))
        .success();
}

#[test]
fn add_task_nonexistent_project_fails() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["add", "Nonexistent: Task"])
        .assert()
        .stderr(predicates::str::contains("E_PROJECT_NOT_FOUND"))
        .failure();
}

#[test]
fn delete_project_with_tasks_fails() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Create project and task
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "add", "Work"])
        .assert()
        .success();

    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["add", "Work: Task"])
        .assert()
        .success();

    // Delete should fail
    todo_cmd()
        .env("TODO_DB_PATH", &db_path)
        .args(["project", "delete", "Work"])
        .assert()
        .stderr(predicates::str::contains("E_PROJECT_HAS_TASKS"))
        .failure();
}
