use std::collections::HashMap;

use crate::cli::StatsArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;
use crate::time_parse::parse_since;

pub fn execute(db: &Database, args: StatsArgs, output: &Output) -> Result<String, TodoError> {
    let since = args.since.map(|s| parse_since(&s)).transpose()?;
    let tags = args.tag.map(|t| vec![t]);

    let filter = TaskFilter {
        tags,
        since,
        limit: Some(10000),
        ..Default::default()
    };

    let tasks = db.list_tasks(&filter)?;

    let mut by_status: HashMap<String, i64> = HashMap::new();
    let mut by_creator: HashMap<String, i64> = HashMap::new();
    let mut by_tag: HashMap<String, i64> = HashMap::new();
    let mut total_duration_secs: i64 = 0;
    let mut duration_count: i64 = 0;

    for task in &tasks {
        *by_status.entry(task.status.to_string()).or_insert(0) += 1;
        *by_creator.entry(task.creator.to_string()).or_insert(0) += 1;

        for tag in &task.tags {
            *by_tag.entry(tag.clone()).or_insert(0) += 1;
        }

        if let (Some(started), Some(finished)) = (task.started_at, task.finished_at) {
            let dur = finished.signed_duration_since(started).num_seconds();
            if dur > 0 {
                total_duration_secs += dur;
                duration_count += 1;
            }
        }
    }

    let avg_minutes = if duration_count > 0 {
        total_duration_secs / duration_count / 60
    } else {
        0
    };

    let by_status_json: serde_json::Map<String, serde_json::Value> = by_status
        .into_iter()
        .map(|(k, v)| (k, serde_json::json!(v)))
        .collect();

    let by_creator_json: serde_json::Map<String, serde_json::Value> = by_creator
        .into_iter()
        .map(|(k, v)| (k, serde_json::json!(v)))
        .collect();

    let by_tag_json: serde_json::Map<String, serde_json::Value> = by_tag
        .into_iter()
        .map(|(k, v)| (k, serde_json::json!(v)))
        .collect();

    let stats = serde_json::json!({
        "total": tasks.len(),
        "by_status": by_status_json,
        "by_creator": by_creator_json,
        "avg_duration_minutes": avg_minutes,
        "by_tag": by_tag_json,
    });

    Ok(output.stats(&stats))
}
