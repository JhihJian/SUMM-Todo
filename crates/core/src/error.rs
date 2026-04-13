use thiserror::Error;

#[derive(Debug, Error)]
pub enum TodoError {
    #[error("Invalid transition from '{from}' to '{to}'")]
    InvalidTransition { from: String, to: String },

    #[error("Result is required when completing a task")]
    ResultRequired,

    #[error("Blocked reason is required when blocking a task")]
    BlockedReasonRequired,

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Queue is empty")]
    QueueEmpty,

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Project has {0} tasks. Delete or move them first.")]
    ProjectHasTasks(i64),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Project already exists: {0}")]
    ProjectExists(String),

    #[error("Sync error: {0}")]
    SyncError(String),

    #[error("Sync server unreachable")]
    SyncServerUnreachable,

    #[error("Sync authentication failed")]
    SyncAuthFailed,
}

impl TodoError {
    pub fn code(&self) -> &'static str {
        match self {
            TodoError::InvalidTransition { .. } => "E_INVALID_TRANSITION",
            TodoError::ResultRequired => "E_RESULT_REQUIRED",
            TodoError::BlockedReasonRequired => "E_BLOCKED_REASON_REQUIRED",
            TodoError::TaskNotFound(_) => "E_TASK_NOT_FOUND",
            TodoError::QueueEmpty => "E_QUEUE_EMPTY",
            TodoError::Database(_) => "E_DATABASE",
            TodoError::InvalidInput(_) => "E_INVALID_INPUT",
            TodoError::ParseError(_) => "E_PARSE_ERROR",
            TodoError::Io(_) => "E_IO",
            TodoError::ProjectHasTasks(_) => "E_PROJECT_HAS_TASKS",
            TodoError::ProjectNotFound(_) => "E_PROJECT_NOT_FOUND",
            TodoError::ProjectExists(_) => "E_PROJECT_EXISTS",
            TodoError::SyncError(_) => "E_SYNC_ERROR",
            TodoError::SyncServerUnreachable => "E_SYNC_SERVER_UNREACHABLE",
            TodoError::SyncAuthFailed => "E_SYNC_AUTH_FAILED",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            TodoError::Database(_) | TodoError::Io(_) => 2,
            _ => 1,
        }
    }
}

pub fn format_error(err: &TodoError) -> String {
    serde_json::json!({
        "error": err.code(),
        "message": err.to_string(),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_match_prd() {
        let cases: Vec<(TodoError, &str)> = vec![
            (
                TodoError::InvalidTransition {
                    from: "Open".into(),
                    to: "Done".into(),
                },
                "E_INVALID_TRANSITION",
            ),
            (TodoError::ResultRequired, "E_RESULT_REQUIRED"),
            (TodoError::BlockedReasonRequired, "E_BLOCKED_REASON_REQUIRED"),
            (
                TodoError::TaskNotFound("abc".into()),
                "E_TASK_NOT_FOUND",
            ),
            (TodoError::QueueEmpty, "E_QUEUE_EMPTY"),
            (
                TodoError::InvalidInput("bad".into()),
                "E_INVALID_INPUT",
            ),
            (
                TodoError::ParseError("oops".into()),
                "E_PARSE_ERROR",
            ),
            (
                TodoError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "nope")),
                "E_IO",
            ),
        ];

        for (err, expected_code) in &cases {
            assert_eq!(err.code(), *expected_code, "Wrong code for {:?}", err);
        }
    }

    #[test]
    fn format_error_produces_valid_json() {
        let err = TodoError::TaskNotFound("abc123".into());
        let json_str = format_error(&err);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("should be valid JSON");

        assert_eq!(parsed["error"], "E_TASK_NOT_FOUND");
        assert_eq!(parsed["message"], "Task not found: abc123");
    }

    #[test]
    fn exit_code_mapping() {
        // User input errors → 1
        assert_eq!(TodoError::ResultRequired.exit_code(), 1);
        assert_eq!(TodoError::BlockedReasonRequired.exit_code(), 1);
        assert_eq!(TodoError::QueueEmpty.exit_code(), 1);
        assert_eq!(
            TodoError::InvalidTransition {
                from: "a".into(),
                to: "b".into()
            }
            .exit_code(),
            1
        );
        assert_eq!(TodoError::TaskNotFound("x".into()).exit_code(), 1);
        assert_eq!(TodoError::InvalidInput("x".into()).exit_code(), 1);
        assert_eq!(TodoError::ParseError("x".into()).exit_code(), 1);

        // System errors → 2
        assert_eq!(
            TodoError::Io(std::io::Error::new(std::io::ErrorKind::Other, "fail")).exit_code(),
            2
        );
    }
}
