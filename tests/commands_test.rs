use chrono::{Utc, Duration};
use todo::cli::*;
use todo::commands;
use todo::db::Database;
use todo::output::Output;
use todo::task::Task;

fn setup() -> (Database, Output) {
    (Database::open_in_memory().unwrap(), Output::json())
}

fn extract_id(json_str: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json_str).unwrap();
    v["id"].as_str().unwrap().to_string()
}

#[test]
fn add_creates_task_and_returns_json() {
    let (db, out) = setup();
    let args = AddArgs {
        title: "Test task".into(),
        pri: Some("high".into()),
        tag: vec!["backend".into()],
        parent: None,
        due: None,
        creator: None,
    };
    let result = commands::add::execute(&db, args, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["title"], "Test task");
    assert_eq!(parsed["priority"], "high");
    assert_eq!(parsed["status"], "pending");
}

#[test]
fn next_claims_highest_priority_task() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Low".into(),
            pri: Some("low".into()),
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    commands::add::execute(
        &db,
        AddArgs {
            title: "High".into(),
            pri: Some("high".into()),
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    let result =
        commands::next::execute(&db, NextArgs { tag: None, pri: None }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["title"], "High");
    assert_eq!(parsed["status"], "in_progress");
}

#[test]
fn done_completes_task() {
    let (db, out) = setup();
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Work".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);

    commands::start::execute(
        &db,
        StartArgs {
            id: id.clone(),
            assignee: None,
        },
        &out,
    )
    .unwrap();
    let result = commands::done::execute(
        &db,
        DoneArgs {
            id: id.clone(),
            result: "Finished".into(),
            artifact: vec!["commit:abc".into()],
            log: None,
        },
        &out,
    )
    .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "done");
    assert_eq!(parsed["result"], "Finished");
}

#[test]
fn block_and_resume() {
    let (db, out) = setup();
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Work".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);

    commands::start::execute(
        &db,
        StartArgs {
            id: id.clone(),
            assignee: None,
        },
        &out,
    )
    .unwrap();
    commands::block::execute(
        &db,
        BlockArgs {
            id: id.clone(),
            reason: "need key".into(),
        },
        &out,
    )
    .unwrap();

    let show = commands::show::execute(&db, ShowArgs { id: id.clone() }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&show).unwrap();
    assert_eq!(parsed["status"], "blocked");

    commands::resume::execute(&db, ResumeArgs { id: id.clone() }, &out).unwrap();
    let show = commands::show::execute(&db, ShowArgs { id: id.clone() }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&show).unwrap();
    assert_eq!(parsed["status"], "in_progress");
}

#[test]
fn cancel_from_pending() {
    let (db, out) = setup();
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Work".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);

    let result = commands::cancel::execute(
        &db,
        CancelArgs {
            id: id.clone(),
            reason: None,
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "cancelled");
}

#[test]
fn show_returns_full_task() {
    let (db, out) = setup();
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Show me".into(),
            pri: None,
            tag: vec!["test".into()],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);

    let result = commands::show::execute(&db, ShowArgs { id }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["title"], "Show me");
    assert!(parsed["tags"].as_array().unwrap().contains(&serde_json::json!("test")));
}

#[test]
fn list_filters_by_status() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Pending".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    let result = commands::list::execute(
        &db,
        ListArgs {
            status: vec!["pending".into()],
            tag: vec![],
            pri: None,
            parent: None,
            creator: None,
            since: None,
            limit: None,
            overdue: false,
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
}

#[test]
fn log_returns_done_tasks() {
    let (db, out) = setup();
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Log me".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);

    commands::start::execute(
        &db,
        StartArgs {
            id: id.clone(),
            assignee: None,
        },
        &out,
    )
    .unwrap();
    commands::done::execute(
        &db,
        DoneArgs {
            id,
            result: "Did it".into(),
            artifact: vec![],
            log: None,
        },
        &out,
    )
    .unwrap();

    let result = commands::log::execute(
        &db,
        LogArgs {
            today: true,
            since: None,
            tag: None,
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.as_array().unwrap().len() >= 1);
}

#[test]
fn stats_returns_counts() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "A".into(),
            pri: Some("high".into()),
            tag: vec!["x".into()],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    commands::add::execute(
        &db,
        AddArgs {
            title: "B".into(),
            pri: Some("medium".into()),
            tag: vec!["x".into()],
            parent: None,
            due: None,
            creator: Some("agent".into()),
        },
        &out,
    )
    .unwrap();

    let result =
        commands::stats::execute(&db, StatsArgs { since: None, tag: None }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    // Check basic counts
    assert_eq!(parsed["total"], 2);
    assert!(parsed["by_status"]["pending"].as_i64().unwrap() >= 2);

    // Check enhanced metrics
    assert!(parsed["overdue"].is_i64());
    assert!(parsed["completion_rate"].is_f64());

    // Check by_priority structure
    assert!(parsed["by_priority"]["high"]["total"].is_i64());
    assert!(parsed["by_priority"]["high"]["done"].is_i64());
    assert!(parsed["by_priority"]["medium"]["total"].is_i64());
    assert!(parsed["by_priority"]["medium"]["done"].is_i64());
    assert!(parsed["by_priority"]["low"]["total"].is_i64());
    assert!(parsed["by_priority"]["low"]["done"].is_i64());

    // Verify priority counts
    assert_eq!(parsed["by_priority"]["high"]["total"], 1);
    assert_eq!(parsed["by_priority"]["medium"]["total"], 1);
    assert_eq!(parsed["by_priority"]["low"]["total"], 0);

    // Check by_status has all statuses
    assert!(parsed["by_status"]["pending"].is_i64());
    assert!(parsed["by_status"]["in_progress"].is_i64());
    assert!(parsed["by_status"]["blocked"].is_i64());
    assert!(parsed["by_status"]["done"].is_i64());
    assert!(parsed["by_status"]["cancelled"].is_i64());
}

#[test]
fn import_creates_tasks_from_json() {
    let (db, out) = setup();
    let json_input = r#"[{"title": "Import A", "priority": "high"}, {"title": "Import B"}]"#;
    let tmpfile = std::env::temp_dir().join("test_import.json");
    std::fs::write(&tmpfile, json_input).unwrap();

    let result = commands::import::execute(
        &db,
        ImportArgs {
            file: tmpfile.to_str().unwrap().into(),
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["imported"], 2);
}

#[test]
fn edit_updates_task_properties() {
    let (db, out) = setup();
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Original".into(),
            pri: Some("low".into()),
            tag: vec!["old".into()],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);

    let result = commands::edit::execute(
        &db,
        EditArgs {
            id: id.clone(),
            title: Some("Updated".into()),
            priority: Some("high".into()),
            tag: vec!["-old".into(), "+new".into()],
            due: None,
        },
        &out,
    )
    .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["title"], "Updated");
    assert_eq!(parsed["priority"], "high");
    assert!(parsed["tags"].as_array().unwrap().contains(&serde_json::json!("new")));
    assert!(!parsed["tags"].as_array().unwrap().contains(&serde_json::json!("old")));
}

#[test]
fn edit_rejects_terminal_states() {
    let (db, out) = setup();
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Task".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);

    // Cancel the task
    commands::cancel::execute(
        &db,
        CancelArgs {
            id: id.clone(),
            reason: None,
        },
        &out,
    )
    .unwrap();

    // Try to edit - should fail
    let result = commands::edit::execute(
        &db,
        EditArgs {
            id: id.clone(),
            title: Some("Updated".into()),
            priority: None,
            tag: vec![],
            due: None,
        },
        &out,
    );
    assert!(result.is_err());
}

#[test]
fn list_overdue_filters_correctly() {
    let (db, out) = setup();

    // Create an overdue task (due 2 days ago)
    let mut task = Task::new("1", "Overdue task");
    task.due = Some(Utc::now() - Duration::days(2));
    db.insert_task(&task).unwrap();

    // Create a non-overdue task (due in 2 days)
    let mut task2 = Task::new("2", "Future task");
    task2.due = Some(Utc::now() + Duration::days(2));
    db.insert_task(&task2).unwrap();

    // Create a task with no due date
    let task3 = Task::new("3", "No due date task");
    db.insert_task(&task3).unwrap();

    let result = commands::list::execute(
        &db,
        ListArgs {
            status: vec![],
            tag: vec![],
            pri: None,
            parent: None,
            creator: None,
            since: None,
            limit: None,
            overdue: true,
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    // Only the overdue task should be returned
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["id"], "1");
}

#[test]
fn undo_reverts_done_to_in_progress() {
    let (db, out) = setup();
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Test".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);
    commands::start::execute(
        &db,
        StartArgs {
            id: id.clone(),
            assignee: None,
        },
        &out,
    )
    .unwrap();
    commands::done::execute(
        &db,
        DoneArgs {
            id: id.clone(),
            result: "Done".into(),
            artifact: vec![],
            log: None,
        },
        &out,
    )
    .unwrap();

    let result = commands::undo::execute(&db, UndoArgs { id: id.clone() }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "in_progress");
    assert!(parsed["result"].is_null());
}

#[test]
fn abandon_reverts_in_progress_to_pending() {
    let (db, out) = setup();
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Test".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);
    commands::start::execute(
        &db,
        StartArgs {
            id: id.clone(),
            assignee: None,
        },
        &out,
    )
    .unwrap();

    let result = commands::abandon::execute(&db, AbandonArgs { id: id.clone() }, &out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "pending");
    assert!(parsed["assignee"].is_null());
}

#[test]
fn search_finds_tasks_by_title() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Implement auth".into(),
            pri: None,
            tag: vec!["backend".into()],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Write tests".into(),
            pri: None,
            tag: vec!["testing".into()],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    let result = commands::search::execute(
        &db,
        SearchArgs {
            query: "auth".into(),
            tag: None,
            status: None,
            regex: false,
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["title"], "Implement auth");
}

#[test]
fn search_with_tag_filter() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Backend auth".into(),
            pri: None,
            tag: vec!["backend".into()],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Frontend auth".into(),
            pri: None,
            tag: vec!["frontend".into()],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    let result = commands::search::execute(
        &db,
        SearchArgs {
            query: "auth".into(),
            tag: Some("backend".into()),
            status: None,
            regex: false,
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["title"], "Backend auth");
}

#[test]
fn search_with_status_filter() {
    let (db, out) = setup();

    // Create and start a task
    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Auth work".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);
    commands::start::execute(
        &db,
        StartArgs {
            id: id.clone(),
            assignee: None,
        },
        &out,
    )
    .unwrap();

    // Create another pending task
    commands::add::execute(
        &db,
        AddArgs {
            title: "Auth planning".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    // Search for "Auth" with in_progress filter
    let result = commands::search::execute(
        &db,
        SearchArgs {
            query: "Auth".into(),
            tag: None,
            status: Some("in_progress".into()),
            regex: false,
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["status"], "in_progress");
}

#[test]
fn search_returns_empty_for_no_match() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Write tests".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    let result = commands::search::execute(
        &db,
        SearchArgs {
            query: "nonexistent".into(),
            tag: None,
            status: None,
            regex: false,
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 0);
}

#[test]
fn export_outputs_json() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Task 1".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    let result = commands::export::execute(
        &db,
        ExportArgs {
            file: None,
            status: vec![],
            tag: vec![],
        },
        &out,
    )
    .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.as_array().unwrap().len() >= 1);
}

#[test]
fn export_to_file() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Task to export".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    let tmpfile = std::env::temp_dir().join("test_export.json");
    let result = commands::export::execute(
        &db,
        ExportArgs {
            file: Some(tmpfile.to_str().unwrap().into()),
            status: vec![],
            tag: vec![],
        },
        &out,
    )
    .unwrap();

    assert!(result.contains("Exported"));
    assert!(result.contains("tasks to"));

    // Verify file contents
    let contents = std::fs::read_to_string(&tmpfile).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
    assert!(parsed.as_array().unwrap().len() >= 1);
}

#[test]
fn export_with_status_filter() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Pending task".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    let add_result = commands::add::execute(
        &db,
        AddArgs {
            title: "Task to complete".into(),
            pri: None,
            tag: vec![],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    let id = extract_id(&add_result);

    // Start and complete the task
    commands::start::execute(
        &db,
        StartArgs {
            id: id.clone(),
            assignee: None,
        },
        &out,
    )
    .unwrap();
    commands::done::execute(
        &db,
        DoneArgs {
            id: id.clone(),
            result: "Done".into(),
            artifact: vec![],
            log: None,
        },
        &out,
    )
    .unwrap();

    // Export only done tasks
    let result = commands::export::execute(
        &db,
        ExportArgs {
            file: None,
            status: vec!["done".into()],
            tag: vec![],
        },
        &out,
    )
    .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let tasks = parsed.as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["status"], "done");
}

#[test]
fn export_with_tag_filter() {
    let (db, out) = setup();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Backend task".into(),
            pri: None,
            tag: vec!["backend".into()],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();
    commands::add::execute(
        &db,
        AddArgs {
            title: "Frontend task".into(),
            pri: None,
            tag: vec!["frontend".into()],
            parent: None,
            due: None,
            creator: None,
        },
        &out,
    )
    .unwrap();

    // Export only backend tasks
    let result = commands::export::execute(
        &db,
        ExportArgs {
            file: None,
            status: vec![],
            tag: vec!["backend".into()],
        },
        &out,
    )
    .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let tasks = parsed.as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Backend task");
}
