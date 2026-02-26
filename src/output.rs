use chrono::Utc;
use crate::db::{Database, ProjectStats};
use crate::error::TodoError;
use crate::task::{Priority, Project, Status, Task};

/// Output format mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputMode {
    /// TOON format (default, token-efficient for LLMs)
    Toon,
    /// JSON format (for backwards compatibility)
    Json,
    /// Human-readable format
    Pretty,
}

pub struct Output {
    mode: OutputMode,
}

impl Output {
    pub fn new(pretty: bool, _toon: bool, json: bool) -> Self {
        let mode = if pretty {
            OutputMode::Pretty
        } else if json {
            OutputMode::Json
        } else {
            // Default to TOON for LLM consumption
            OutputMode::Toon
        };
        Self { mode }
    }

    /// Create output with JSON mode (for testing)
    pub fn json() -> Self {
        Self { mode: OutputMode::Json }
    }

    pub fn task(&self, task: &Task) -> String {
        match self.mode {
            OutputMode::Toon => self.toon_task(task),
            OutputMode::Json => {
                serde_json::to_string_pretty(task).expect("task serialization should not fail")
            }
            OutputMode::Pretty => self.pretty_task(task),
        }
    }

    pub fn task_list(&self, tasks: &[Task]) -> String {
        match self.mode {
            OutputMode::Toon => self.toon_task_list(tasks),
            OutputMode::Json => {
                serde_json::to_string_pretty(tasks).expect("task list serialization should not fail")
            }
            OutputMode::Pretty => tasks
                .iter()
                .map(|t| self.pretty_task(t))
                .collect::<Vec<_>>()
                .join("\n\n"),
        }
    }

    pub fn task_list_grouped(&self, tasks: &[Task], db: &Database) -> Result<String, TodoError> {
        // For non-Pretty modes, just return the regular task list (no grouping)
        if self.mode != OutputMode::Pretty {
            return Ok(self.task_list(tasks));
        }

        // Group tasks by project_id
        let mut groups: std::collections::HashMap<Option<String>, Vec<&Task>> =
            std::collections::HashMap::new();

        for task in tasks {
            groups.entry(task.project_id.clone()).or_default().push(task);
        }

        let mut result = String::new();

        // Sort groups: projects with names first, then no-project tasks
        let mut project_ids: Vec<_> = groups.keys().collect();
        project_ids.sort_by(|a, b| {
            match (a, b) {
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(a_id), Some(b_id)) => a_id.cmp(b_id),
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        for project_id in project_ids {
            let group_tasks = groups.get(&project_id).unwrap();

            let project_name = if let Some(ref pid) = project_id {
                db.get_project(pid)?
                    .map(|p| p.name)
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "(no project)".to_string()
            };

            result.push_str(&format!("=== {} ({}) ===\n", project_name, group_tasks.len()));

            for task in group_tasks {
                result.push_str(&self.pretty_task(task));
                result.push('\n');
            }
            result.push('\n');
        }

        Ok(result.trim_end().to_string())
    }

    pub fn log(&self, tasks: &[Task]) -> String {
        match self.mode {
            OutputMode::Toon => self.toon_task_list(tasks),
            OutputMode::Json => {
                serde_json::to_string_pretty(tasks).expect("log serialization should not fail")
            }
            OutputMode::Pretty => {
                let mut out = String::from("Completed tasks:\n---\n");
                for t in tasks {
                    out.push_str(&format!("● [{}] {}\n", t.id, t.title));
                    if let Some(ref result) = t.result {
                        out.push_str(&format!("  → {}\n", result));
                    }
                    if !t.artifacts.is_empty() {
                        out.push_str(&format!("  📎 {}\n", t.artifacts.join(", ")));
                    }
                }
                out
            }
        }
    }

    pub fn stats(&self, stats: &serde_json::Value) -> String {
        match self.mode {
            OutputMode::Toon => toon_format::encode_default(stats).unwrap_or_else(|e| {
                format!("error: failed to encode stats to TOON: {}", e)
            }),
            OutputMode::Json | OutputMode::Pretty => {
                serde_json::to_string_pretty(stats).expect("stats serialization should not fail")
            }
        }
    }

    fn toon_task(&self, task: &Task) -> String {
        toon_format::encode_default(task).unwrap_or_else(|e| {
            format!("error: failed to encode task to TOON: {}", e)
        })
    }

    fn toon_task_list(&self, tasks: &[Task]) -> String {
        // Convert slice to Vec since toon_format requires Sized types
        let tasks_vec: Vec<&Task> = tasks.iter().collect();
        toon_format::encode_default(&tasks_vec).unwrap_or_else(|e| {
            format!("error: failed to encode tasks to TOON: {}", e)
        })
    }

    fn pretty_task(&self, task: &Task) -> String {
        let status_icon = match task.status {
            Status::Pending => "○",
            Status::InProgress => "◐",
            Status::Blocked => "⊘",
            Status::Done => "●",
            Status::Cancelled => "✕",
        };

        let pri_icon = match task.priority {
            Priority::High => "!",
            Priority::Medium => "·",
            Priority::Low => "_",
        };

        let tags = if task.tags.is_empty() {
            String::new()
        } else {
            task.tags
                .iter()
                .map(|t| format!("#{}", t))
                .collect::<Vec<_>>()
                .join(" ")
        };

        let mut line = format!("{} {} {} [{}]", status_icon, pri_icon, task.id, task.title);
        if !tags.is_empty() {
            line.push(' ');
            line.push_str(&tags);
        }

        // Show overdue indicator for pending tasks past their due date
        if task.status == Status::Pending {
            if let Some(due) = task.due {
                if due < Utc::now() {
                    let days = (Utc::now() - due).num_days();
                    line.push_str(&format!(" \u{26a0}\u{fe0f} {} days overdue", days));
                }
            }
        }

        // Show blocked reason for blocked tasks
        if task.status == Status::Blocked {
            if let Some(ref reason) = task.blocked_reason {
                line.push_str(&format!("\n  \u{26a0} {}", reason));
            }
        }

        // Show content if present (only in show context, not list)
        if let Some(ref content) = task.content {
            if !content.is_empty() {
                line.push_str("\n\n详细内容:\n");
                for content_line in content.lines() {
                    line.push_str(&format!("  {}\n", content_line));
                }
                // Remove trailing newline
                line.pop();
            }
        }

        line
    }

    pub fn project(&self, project: &Project) -> String {
        match self.mode {
            OutputMode::Toon => toon_format::encode_default(project).unwrap_or_else(|e| {
                format!("error: failed to encode project to TOON: {}", e)
            }),
            OutputMode::Json => {
                serde_json::to_string_pretty(project).expect("project serialization should not fail")
            }
            OutputMode::Pretty => self.pretty_project(project),
        }
    }

    pub fn project_list_item(&self, project: &Project, stats: &ProjectStats) -> String {
        match self.mode {
            OutputMode::Toon | OutputMode::Json => {
                let obj = serde_json::json!({
                    "id": project.id,
                    "name": project.name,
                    "tasks": stats.total,
                    "task_breakdown": {
                        "pending": stats.pending,
                        "in_progress": stats.in_progress,
                        "done": stats.done,
                    }
                });
                toon_format::encode_default(&obj).unwrap_or_else(|e| {
                    format!("error: failed to encode project to TOON: {}", e)
                })
            }
            OutputMode::Pretty => {
                format!(
                    "id: \"{}\"\nname: {}\ntasks: {} ({} pending, {} in_progress, {} done)",
                    project.id,
                    project.name,
                    stats.total,
                    stats.pending,
                    stats.in_progress,
                    stats.done
                )
            }
        }
    }

    pub fn project_detail(&self, project: &Project, stats: &ProjectStats, recent_tasks: &[Task]) -> String {
        match self.mode {
            OutputMode::Toon | OutputMode::Json => {
                let tasks_json: Vec<_> = recent_tasks.iter().map(|t| serde_json::json!({
                    "id": t.id,
                    "title": t.title,
                    "status": t.status.to_string(),
                })).collect();

                let obj = serde_json::json!({
                    "name": project.name,
                    "description": project.description,
                    "created": project.created_at.format("%Y-%m-%d").to_string(),
                    "statistics": {
                        "total": stats.total,
                        "pending": stats.pending,
                        "in_progress": stats.in_progress,
                        "blocked": stats.blocked,
                        "done": stats.done,
                    },
                    "recent_tasks": tasks_json,
                });
                toon_format::encode_default(&obj).unwrap_or_else(|e| {
                    format!("error: failed to encode project to TOON: {}", e)
                })
            }
            OutputMode::Pretty => {
                let mut out = format!(
                    "name: {}\ndescription: {}\ncreated: {}\n\nstatistics:\n  total: {}\n  pending: {}\n  in_progress: {}\n  blocked: {}\n  done: {}\n\nrecent tasks:\n",
                    project.name,
                    project.description.as_deref().unwrap_or("N/A"),
                    project.created_at.format("%Y-%m-%d"),
                    stats.total,
                    stats.pending,
                    stats.in_progress,
                    stats.blocked,
                    stats.done,
                );

                for task in recent_tasks {
                    out.push_str(&format!("  {}\n", self.pretty_task(task)));
                }

                if recent_tasks.is_empty() {
                    out.push_str("  (no tasks)\n");
                }

                out.trim_end().to_string()
            }
        }
    }

    pub fn project_deleted(&self, project: &Project) -> String {
        match self.mode {
            OutputMode::Toon | OutputMode::Json => {
                let obj = serde_json::json!({
                    "deleted": true,
                    "name": project.name,
                });
                toon_format::encode_default(&obj).unwrap_or_else(|e| {
                    format!("error: failed to encode to TOON: {}", e)
                })
            }
            OutputMode::Pretty => {
                format!("Project '{}' deleted.", project.name)
            }
        }
    }

    fn pretty_project(&self, project: &Project) -> String {
        let mut out = format!("id: \"{}\"\nname: {}", project.id, project.name);
        if let Some(ref desc) = project.description {
            out.push_str(&format!("\ndescription: {}", desc));
        }
        out
    }
}

pub fn output_error(err: &TodoError) -> String {
    let error_json = serde_json::json!({
        "error": err.code(),
        "message": err.to_string(),
    });
    // Errors always output in TOON format for consistency
    toon_format::encode_default(&error_json).unwrap_or_else(|_e| {
        format!("error: {}\nmessage: {}", err.code(), err)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;

    #[test]
    fn toon_output_is_valid() {
        let task = Task::new("abc123", "Write tests");
        let out = Output::new(false, false, false);
        let toon_str = out.task(&task);
        // TOON format should contain key-value pairs
        assert!(toon_str.contains("id:"), "TOON should contain 'id:' key");
        assert!(toon_str.contains("abc123"), "TOON should contain id value");
    }

    #[test]
    fn toon_list_is_valid() {
        let tasks = vec![
            Task::new("1", "First"),
            Task::new("2", "Second"),
            Task::new("3", "Third"),
        ];
        let out = Output::new(false, false, false);
        let toon_str = out.task_list(&tasks);
        // TOON array format uses [n] notation
        assert!(toon_str.contains("["), "TOON array should use bracket notation");
    }

    #[test]
    fn pretty_output_contains_status_icon() {
        let task = Task::new("abc123", "Write tests");
        let out = Output::new(true, false, false);
        let result = out.task(&task);
        assert!(result.contains("○"), "should contain pending icon");
        assert!(result.contains("abc123"), "should contain id");
        assert!(result.contains("Write tests"), "should contain title");
    }

    #[test]
    fn json_output_is_valid() {
        let task = Task::new("abc123", "Write tests");
        let out = Output::json();
        let json_str = out.task(&task);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("should be valid JSON");
        assert_eq!(parsed["id"], "abc123");
        assert_eq!(parsed["status"], "pending");
    }

    #[test]
    fn error_output_is_toon() {
        let err = TodoError::TaskNotFound("xyz".into());
        let toon_str = output_error(&err);
        // TOON format uses key: value syntax
        assert!(toon_str.contains("error:"), "TOON error should contain 'error:' key");
        assert!(toon_str.contains("E_TASK_NOT_FOUND"), "TOON error should contain error code");
    }

    #[test]
    fn toon_skips_empty_fields() {
        let task = Task::new("abc", "Simple task");
        let out = Output::new(false, false, false);
        let toon_str = out.task(&task);

        // Should NOT contain empty fields
        assert!(!toon_str.contains("tags[0]:"), "Should not show empty tags");
        assert!(!toon_str.contains("artifacts[0]:"), "Should not show empty artifacts");
        assert!(!toon_str.contains("parent_id: null"), "Should not show null parent_id");
        assert!(!toon_str.contains("blocked_reason: null"), "Should not show null blocked_reason");

        // Should contain required fields
        assert!(toon_str.contains("id:"), "Should show id");
        assert!(toon_str.contains("title:"), "Should show title");
        assert!(toon_str.contains("status:"), "Should show status");
    }
}
