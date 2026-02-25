use std::collections::HashMap;

use chrono::Utc;

use crate::cli::StatsArgs;
use crate::db::{Database, TaskFilter};
use crate::error::TodoError;
use crate::output::Output;
use crate::task::{Priority, Status};
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

    let total = tasks.len();
    let done_count = tasks.iter().filter(|t| t.status == Status::Done).count();
    let cancelled_count = tasks
        .iter()
        .filter(|t| t.status == Status::Cancelled)
        .count();

    // Count overdue tasks: due < now AND status != done AND status != cancelled
    let now = Utc::now();
    let overdue_count = tasks
        .iter()
        .filter(|t| {
            t.due.map(|d| d < now).unwrap_or(false)
                && t.status != Status::Done
                && t.status != Status::Cancelled
        })
        .count();

    // Completion rate = done / (total - cancelled)
    let completion_rate = if total > cancelled_count {
        done_count as f64 / (total - cancelled_count) as f64
    } else {
        0.0
    };

    // Group by priority with total and done counts
    let mut by_priority: HashMap<String, (i64, i64)> = HashMap::new(); // (total, done)
    for task in &tasks {
        let priority_key = task.priority.to_string();
        let entry = by_priority.entry(priority_key).or_insert((0, 0));
        entry.0 += 1;
        if task.status == Status::Done {
            entry.1 += 1;
        }
    }

    // Ensure all priorities are present in output
    for priority in [Priority::High, Priority::Medium, Priority::Low] {
        by_priority
            .entry(priority.to_string())
            .or_insert((0, 0));
    }

    let by_priority_json: serde_json::Map<String, serde_json::Value> = by_priority
        .into_iter()
        .map(|(k, (total, done))| {
            (
                k,
                serde_json::json!({
                    "total": total,
                    "done": done
                }),
            )
        })
        .collect();

    // Group by status with counts
    let mut by_status: HashMap<String, i64> = HashMap::new();
    for task in &tasks {
        *by_status.entry(task.status.to_string()).or_insert(0) += 1;
    }

    // Ensure all statuses are present in output
    for status in [
        Status::Pending,
        Status::InProgress,
        Status::Blocked,
        Status::Done,
        Status::Cancelled,
    ] {
        by_status.entry(status.to_string()).or_insert(0);
    }

    let by_status_json: serde_json::Map<String, serde_json::Value> = by_status
        .into_iter()
        .map(|(k, v)| (k, serde_json::json!(v)))
        .collect();

    let stats = serde_json::json!({
        "total": total,
        "overdue": overdue_count,
        "completion_rate": completion_rate,
        "by_priority": by_priority_json,
        "by_status": by_status_json
    });

    Ok(output.stats(&stats))
}
