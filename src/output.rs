use crate::error::TodoError;
use crate::task::{Priority, Status, Task};

pub struct Output {
    pretty: bool,
}

impl Output {
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }

    pub fn task(&self, task: &Task) -> String {
        if self.pretty {
            self.pretty_task(task)
        } else {
            serde_json::to_string_pretty(task).expect("task serialization should not fail")
        }
    }

    pub fn task_list(&self, tasks: &[Task]) -> String {
        if self.pretty {
            tasks
                .iter()
                .map(|t| self.pretty_task(t))
                .collect::<Vec<_>>()
                .join("\n\n")
        } else {
            serde_json::to_string_pretty(tasks).expect("task list serialization should not fail")
        }
    }

    pub fn log(&self, tasks: &[Task]) -> String {
        if self.pretty {
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
        } else {
            serde_json::to_string_pretty(tasks).expect("log serialization should not fail")
        }
    }

    pub fn stats(&self, stats: &serde_json::Value) -> String {
        serde_json::to_string_pretty(stats).expect("stats serialization should not fail")
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
        line
    }
}

pub fn output_error(err: &TodoError) -> String {
    serde_json::json!({
        "error": err.code(),
        "message": err.to_string(),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;

    #[test]
    fn json_output_is_valid_json() {
        let task = Task::new("abc123", "Write tests");
        let out = Output::new(false);
        let json_str = out.task(&task);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("should be valid JSON");
        assert_eq!(parsed["id"], "abc123");
        assert_eq!(parsed["status"], "pending");
    }

    #[test]
    fn json_list_is_array() {
        let tasks = vec![
            Task::new("1", "First"),
            Task::new("2", "Second"),
            Task::new("3", "Third"),
        ];
        let out = Output::new(false);
        let json_str = out.task_list(&tasks);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("should be valid JSON");
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 3);
    }

    #[test]
    fn pretty_output_contains_status_icon() {
        let task = Task::new("abc123", "Write tests");
        let out = Output::new(true);
        let result = out.task(&task);
        assert!(result.contains("○"), "should contain pending icon");
        assert!(result.contains("abc123"), "should contain id");
        assert!(result.contains("Write tests"), "should contain title");
    }

    #[test]
    fn error_output_is_json() {
        let err = TodoError::TaskNotFound("xyz".into());
        let json_str = output_error(&err);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("should be valid JSON");
        assert_eq!(parsed["error"], "E_TASK_NOT_FOUND");
    }
}
