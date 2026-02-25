use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::TodoError;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Status::Pending => "pending",
            Status::InProgress => "in_progress",
            Status::Blocked => "blocked",
            Status::Done => "done",
            Status::Cancelled => "cancelled",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Status {
    type Err = TodoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Status::Pending),
            "in_progress" => Ok(Status::InProgress),
            "blocked" => Ok(Status::Blocked),
            "done" => Ok(Status::Done),
            "cancelled" => Ok(Status::Cancelled),
            _ => Err(TodoError::ParseError(format!("unknown status: {}", s))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Creator {
    Human,
    Agent,
}

impl fmt::Display for Creator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Creator::Human => "human",
            Creator::Agent => "agent",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Creator {
    type Err = TodoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "human" => Ok(Creator::Human),
            "agent" => Ok(Creator::Agent),
            _ => Err(TodoError::ParseError(format!("unknown creator: {}", s))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Priority::High => "high",
            Priority::Medium => "medium",
            Priority::Low => "low",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Priority {
    type Err = TodoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "high" => Ok(Priority::High),
            "medium" => Ok(Priority::Medium),
            "low" => Ok(Priority::Low),
            _ => Err(TodoError::ParseError(format!("unknown priority: {}", s))),
        }
    }
}

// ---------------------------------------------------------------------------
// TransitionContext
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct TransitionContext {
    pub assignee: Option<Creator>,
    pub result: Option<String>,
    pub artifacts: Option<Vec<String>>,
    pub log: Option<String>,
    pub blocked_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub creator: Creator,
    pub created_at: DateTime<Utc>,
    pub priority: Priority,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<DateTime<Utc>>,
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<Creator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub artifacts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            creator: Creator::Human,
            created_at: Utc::now(),
            priority: Priority::Medium,
            tags: Vec::new(),
            parent_id: None,
            due: None,
            status: Status::Pending,
            assignee: None,
            blocked_reason: None,
            result: None,
            artifacts: Vec::new(),
            log: None,
            started_at: None,
            finished_at: None,
        }
    }

    pub fn transition(
        &mut self,
        target: Status,
        ctx: TransitionContext,
    ) -> Result<(), TodoError> {
        // Idempotent: already in target state
        if self.status == target {
            return Ok(());
        }

        match (&self.status, &target) {
            // pending -> in_progress
            (Status::Pending, Status::InProgress) => {
                self.assignee = ctx.assignee;
                self.started_at = Some(Utc::now());
            }
            // pending -> cancelled
            (Status::Pending, Status::Cancelled) => {
                self.finished_at = Some(Utc::now());
            }
            // in_progress -> done
            (Status::InProgress, Status::Done) => {
                let result = ctx
                    .result
                    .ok_or(TodoError::ResultRequired)?;
                self.result = Some(result);
                self.artifacts = ctx.artifacts.unwrap_or_default();
                self.log = ctx.log;
                self.finished_at = Some(Utc::now());
            }
            // in_progress -> blocked
            (Status::InProgress, Status::Blocked) => {
                let reason = ctx
                    .blocked_reason
                    .ok_or(TodoError::BlockedReasonRequired)?;
                self.blocked_reason = Some(reason);
            }
            // in_progress -> cancelled
            (Status::InProgress, Status::Cancelled) => {
                self.finished_at = Some(Utc::now());
            }
            // blocked -> in_progress
            (Status::Blocked, Status::InProgress) => {
                self.blocked_reason = None;
            }
            // blocked -> cancelled
            (Status::Blocked, Status::Cancelled) => {
                self.blocked_reason = None;
                self.finished_at = Some(Utc::now());
            }
            // done -> in_progress (undo)
            (Status::Done, Status::InProgress) => {
                self.result = None;
                self.artifacts = Vec::new();
                self.log = None;
                self.finished_at = None;
            }
            // in_progress -> pending (abandon)
            (Status::InProgress, Status::Pending) => {
                self.assignee = None;
                self.started_at = None;
            }
            // All other transitions are invalid
            _ => {
                return Err(TodoError::InvalidTransition {
                    from: self.status.to_string(),
                    to: target.to_string(),
                });
            }
        }

        self.status = target;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_has_correct_defaults() {
        let task = Task::new("1", "Test task");
        assert_eq!(task.id, "1");
        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, Status::Pending);
        assert_eq!(task.priority, Priority::Medium);
        assert_eq!(task.creator, Creator::Human);
        assert!(task.assignee.is_none());
        assert!(task.parent_id.is_none());
        assert!(task.due.is_none());
        assert!(task.blocked_reason.is_none());
        assert!(task.result.is_none());
        assert!(task.log.is_none());
        assert!(task.started_at.is_none());
        assert!(task.finished_at.is_none());
        assert!(task.tags.is_empty());
        assert!(task.artifacts.is_empty());
    }

    #[test]
    fn pending_to_in_progress() {
        let mut task = Task::new("1", "Test");
        let ctx = TransitionContext {
            assignee: Some(Creator::Agent),
            ..Default::default()
        };
        task.transition(Status::InProgress, ctx).unwrap();
        assert_eq!(task.status, Status::InProgress);
        assert_eq!(task.assignee, Some(Creator::Agent));
        assert!(task.started_at.is_some());
    }

    #[test]
    fn in_progress_to_done_requires_result() {
        let mut task = Task::new("1", "Test");
        task.transition(Status::InProgress, TransitionContext::default())
            .unwrap();

        let err = task
            .transition(Status::Done, TransitionContext::default())
            .unwrap_err();
        assert!(matches!(err, TodoError::ResultRequired));
    }

    #[test]
    fn in_progress_to_done_with_result() {
        let mut task = Task::new("1", "Test");
        task.transition(Status::InProgress, TransitionContext::default())
            .unwrap();

        let ctx = TransitionContext {
            result: Some("completed successfully".into()),
            artifacts: Some(vec!["output.txt".into()]),
            log: Some("all good".into()),
            ..Default::default()
        };
        task.transition(Status::Done, ctx).unwrap();
        assert_eq!(task.status, Status::Done);
        assert_eq!(task.result.as_deref(), Some("completed successfully"));
        assert_eq!(task.artifacts, vec!["output.txt".to_string()]);
        assert_eq!(task.log.as_deref(), Some("all good"));
        assert!(task.finished_at.is_some());
    }

    #[test]
    fn block_requires_reason() {
        let mut task = Task::new("1", "Test");
        task.transition(Status::InProgress, TransitionContext::default())
            .unwrap();

        let err = task
            .transition(Status::Blocked, TransitionContext::default())
            .unwrap_err();
        assert!(matches!(err, TodoError::BlockedReasonRequired));
    }

    #[test]
    fn block_and_resume() {
        let mut task = Task::new("1", "Test");
        task.transition(Status::InProgress, TransitionContext::default())
            .unwrap();

        // Block
        let ctx = TransitionContext {
            blocked_reason: Some("waiting on dependency".into()),
            ..Default::default()
        };
        task.transition(Status::Blocked, ctx).unwrap();
        assert_eq!(task.status, Status::Blocked);
        assert_eq!(
            task.blocked_reason.as_deref(),
            Some("waiting on dependency")
        );

        // Resume
        task.transition(Status::InProgress, TransitionContext::default())
            .unwrap();
        assert_eq!(task.status, Status::InProgress);
        assert!(task.blocked_reason.is_none());
    }

    #[test]
    fn cancel_from_any_non_terminal_state() {
        // From pending
        let mut t1 = Task::new("1", "Test");
        t1.transition(Status::Cancelled, TransitionContext::default())
            .unwrap();
        assert_eq!(t1.status, Status::Cancelled);
        assert!(t1.finished_at.is_some());

        // From in_progress
        let mut t2 = Task::new("2", "Test");
        t2.transition(Status::InProgress, TransitionContext::default())
            .unwrap();
        t2.transition(Status::Cancelled, TransitionContext::default())
            .unwrap();
        assert_eq!(t2.status, Status::Cancelled);
        assert!(t2.finished_at.is_some());

        // From blocked
        let mut t3 = Task::new("3", "Test");
        t3.transition(Status::InProgress, TransitionContext::default())
            .unwrap();
        let ctx = TransitionContext {
            blocked_reason: Some("reason".into()),
            ..Default::default()
        };
        t3.transition(Status::Blocked, ctx).unwrap();
        t3.transition(Status::Cancelled, TransitionContext::default())
            .unwrap();
        assert_eq!(t3.status, Status::Cancelled);
        assert!(t3.blocked_reason.is_none());
        assert!(t3.finished_at.is_some());
    }

    #[test]
    fn terminal_states_reject_transitions() {
        // Done -> Pending is invalid (must go through InProgress via undo)
        let mut t1 = Task::new("1", "Test");
        t1.transition(Status::InProgress, TransitionContext::default())
            .unwrap();
        let ctx = TransitionContext {
            result: Some("done".into()),
            ..Default::default()
        };
        t1.transition(Status::Done, ctx).unwrap();

        let err = t1
            .transition(Status::Pending, TransitionContext::default())
            .unwrap_err();
        assert!(matches!(err, TodoError::InvalidTransition { .. }));

        // Done -> InProgress is allowed (undo)
        t1.transition(Status::InProgress, TransitionContext::default())
            .unwrap();
        assert_eq!(t1.status, Status::InProgress);
        assert!(t1.result.is_none());

        // Cancelled rejects all transitions
        let mut t2 = Task::new("2", "Test");
        t2.transition(Status::Cancelled, TransitionContext::default())
            .unwrap();

        let err = t2
            .transition(Status::Pending, TransitionContext::default())
            .unwrap_err();
        assert!(matches!(err, TodoError::InvalidTransition { .. }));

        let err = t2
            .transition(Status::InProgress, TransitionContext::default())
            .unwrap_err();
        assert!(matches!(err, TodoError::InvalidTransition { .. }));
    }

    #[test]
    fn idempotent_same_state() {
        let mut task = Task::new("1", "Test");
        assert_eq!(task.status, Status::Pending);
        // pending -> pending should be Ok
        task.transition(Status::Pending, TransitionContext::default())
            .unwrap();
        assert_eq!(task.status, Status::Pending);
    }

    #[test]
    fn illegal_pending_to_done() {
        let mut task = Task::new("1", "Test");
        let ctx = TransitionContext {
            result: Some("skipped".into()),
            ..Default::default()
        };
        let err = task.transition(Status::Done, ctx).unwrap_err();
        assert!(matches!(err, TodoError::InvalidTransition { .. }));
    }

    #[test]
    fn undo_reverts_done_to_in_progress() {
        let mut task = Task::new("1", "Test");
        task.transition(Status::InProgress, TransitionContext::default())
            .unwrap();
        let ctx = TransitionContext {
            result: Some("completed".into()),
            artifacts: Some(vec!["file.txt".into()]),
            log: Some("done".into()),
            ..Default::default()
        };
        task.transition(Status::Done, ctx).unwrap();
        assert_eq!(task.status, Status::Done);
        assert!(task.result.is_some());
        assert!(!task.artifacts.is_empty());
        assert!(task.log.is_some());
        assert!(task.finished_at.is_some());

        // Undo
        task.transition(Status::InProgress, TransitionContext::default())
            .unwrap();
        assert_eq!(task.status, Status::InProgress);
        assert!(task.result.is_none());
        assert!(task.artifacts.is_empty());
        assert!(task.log.is_none());
        assert!(task.finished_at.is_none());
    }

    #[test]
    fn abandon_reverts_in_progress_to_pending() {
        let mut task = Task::new("1", "Test");
        let ctx = TransitionContext {
            assignee: Some(Creator::Agent),
            ..Default::default()
        };
        task.transition(Status::InProgress, ctx).unwrap();
        assert_eq!(task.status, Status::InProgress);
        assert!(task.assignee.is_some());
        assert!(task.started_at.is_some());

        // Abandon
        task.transition(Status::Pending, TransitionContext::default())
            .unwrap();
        assert_eq!(task.status, Status::Pending);
        assert!(task.assignee.is_none());
        assert!(task.started_at.is_none());
    }
}
