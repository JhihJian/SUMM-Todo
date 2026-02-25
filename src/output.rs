use chrono::Utc;
use crate::error::TodoError;
use crate::task::{Priority, Status, Task};

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

        line
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
