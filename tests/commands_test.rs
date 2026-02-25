use todo::cli::*;
use todo::commands;
use todo::db::Database;
use todo::output::Output;

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
            pri: None,
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
            pri: None,
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
    assert_eq!(parsed["total"], 2);
    assert!(parsed["by_status"]["pending"].as_i64().unwrap() >= 2);
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
            json: tmpfile.to_str().unwrap().into(),
        },
        &out,
    )
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["imported"], 2);
}
