use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct PushRequest {
    pub device_id: String,
    #[serde(default)]
    pub tasks: Vec<Value>,
    #[serde(default)]
    pub deleted_ids: Vec<String>,
    #[serde(default)]
    pub projects: Vec<Value>,
    #[serde(default)]
    pub deleted_project_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Conflict {
    pub id: String,
    pub client_updated_at: String,
    pub server_updated_at: String,
    pub server_data: Value,
}

#[derive(Debug, Serialize)]
pub struct PushResponse {
    pub ok: bool,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub device_id: String,
    pub since: String,
}

#[derive(Debug, Serialize)]
pub struct PullResponse {
    pub tasks: Vec<Value>,
    pub deleted_ids: Vec<String>,
    pub projects: Vec<Value>,
    pub deleted_project_ids: Vec<String>,
    pub server_time: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub total_tasks: i64,
    pub last_modified: Option<String>,
    pub devices: Vec<DeviceEntry>,
}

#[derive(Debug, Serialize)]
pub struct DeviceEntry {
    pub device_id: String,
    pub last_sync: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn push(
    State(state): State<AppState>,
    Json(req): Json<PushRequest>,
) -> Result<(StatusCode, Json<PushResponse>), (StatusCode, Json<Value>)> {
    let db = &state.db;
    let mut conflicts = Vec::new();

    // Upsert tasks
    for task in &req.tasks {
        let id = task["id"]
            .as_str()
            .ok_or_else(|| bad_request("Task missing 'id' field"))?;
        let updated_at = task["updated_at"]
            .as_str()
            .ok_or_else(|| bad_request(format!("Task {} missing 'updated_at' field", id)))?
            .to_string();

        match db.upsert_task(id, &task.to_string(), &updated_at, &req.device_id) {
            Ok(None) => {}
            Ok(Some(server_updated_at)) => {
                if let Some((server_data, _)) = db.get_task_data(id).unwrap_or(None) {
                    conflicts.push(Conflict {
                        id: id.to_string(),
                        client_updated_at: updated_at,
                        server_updated_at,
                        server_data,
                    });
                }
            }
            Err(e) => return Err(internal_error(e.to_string())),
        }
    }

    // Soft-delete tasks
    for id in &req.deleted_ids {
        let now = Utc::now().to_rfc3339();
        match db.soft_delete_task(id, &now, &req.device_id) {
            Ok(None) => {}
            Ok(Some(server_updated_at)) => {
                if let Some((server_data, _)) = db.get_task_data(id).unwrap_or(None) {
                    conflicts.push(Conflict {
                        id: id.clone(),
                        client_updated_at: now,
                        server_updated_at,
                        server_data,
                    });
                }
            }
            Err(e) => return Err(internal_error(e.to_string())),
        }
    }

    // Upsert projects
    for project in &req.projects {
        let id = project["id"]
            .as_str()
            .ok_or_else(|| bad_request("Project missing 'id' field"))?;
        let updated_at = project["updated_at"]
            .as_str()
            .ok_or_else(|| bad_request(format!("Project {} missing 'updated_at' field", id)))?
            .to_string();

        match db.upsert_project(id, &project.to_string(), &updated_at, &req.device_id) {
            Ok(None) => {}
            Ok(Some(server_updated_at)) => {
                if let Some((server_data, _)) = db.get_project_data(id).unwrap_or(None) {
                    conflicts.push(Conflict {
                        id: id.to_string(),
                        client_updated_at: updated_at,
                        server_updated_at,
                        server_data,
                    });
                }
            }
            Err(e) => return Err(internal_error(e.to_string())),
        }
    }

    // Soft-delete projects
    for id in &req.deleted_project_ids {
        let now = Utc::now().to_rfc3339();
        match db.soft_delete_project(id, &now, &req.device_id) {
            Ok(None) => {}
            Ok(Some(server_updated_at)) => {
                if let Some((server_data, _)) = db.get_project_data(id).unwrap_or(None) {
                    conflicts.push(Conflict {
                        id: id.clone(),
                        client_updated_at: now,
                        server_updated_at,
                        server_data,
                    });
                }
            }
            Err(e) => return Err(internal_error(e.to_string())),
        }
    }

    Ok((
        StatusCode::OK,
        Json(PushResponse {
            ok: conflicts.is_empty(),
            conflicts,
        }),
    ))
}

pub async fn pull(
    State(state): State<AppState>,
    Json(req): Json<PullRequest>,
) -> Result<(StatusCode, Json<PullResponse>), (StatusCode, Json<Value>)> {
    let db = &state.db;
    let server_time = Utc::now().to_rfc3339();

    let changes = db
        .get_changes_since(&req.since)
        .map_err(|e| internal_error(e.to_string()))?;

    let mut tasks = Vec::new();
    let mut deleted_ids = Vec::new();
    let mut projects = Vec::new();
    let mut deleted_project_ids = Vec::new();

    for change in changes {
        if change.deleted {
            // Deleted entry. Check tasks table first, then projects.
            if db.get_task_data(&change.id).unwrap_or(None).is_some() {
                deleted_ids.push(change.id);
            } else {
                deleted_project_ids.push(change.id);
            }
        } else {
            // Active entry. Check tasks table first, then projects.
            if db.get_task_data(&change.id).unwrap_or(None).is_some() {
                tasks.push(change.data);
            } else {
                projects.push(change.data);
            }
        }
    }

    // Record device sync
    db.record_device_sync(&req.device_id, &server_time)
        .map_err(|e| internal_error(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(PullResponse {
            tasks,
            deleted_ids,
            projects,
            deleted_project_ids,
            server_time,
        }),
    ))
}

pub async fn status(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<StatusResponse>), (StatusCode, Json<Value>)> {
    let info = state
        .db
        .get_status()
        .map_err(|e| internal_error(e.to_string()))?;

    let devices = info
        .devices
        .into_iter()
        .map(|d| DeviceEntry {
            device_id: d.device_id,
            last_sync: d.last_sync,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(StatusResponse {
            total_tasks: info.total_tasks,
            last_modified: info.last_modified,
            devices,
        }),
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn bad_request(msg: impl Into<String>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": "E_BAD_REQUEST",
            "message": msg.into(),
        })),
    )
}

fn internal_error(msg: impl Into<String>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": "E_INTERNAL",
            "message": msg.into(),
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::SyncDb;
    use std::sync::Arc;

    fn test_state() -> AppState {
        let db = SyncDb::open_in_memory().unwrap();
        AppState {
            db: Arc::new(db),
            api_key: "testkey".to_string(),
        }
    }

    #[tokio::test]
    async fn push_inserts_tasks() {
        let state = test_state();
        let req = PushRequest {
            device_id: "dev1".to_string(),
            tasks: vec![serde_json::json!({
                "id": "t1",
                "title": "Test task",
                "updated_at": "2026-01-01T00:00:00Z",
            })],
            deleted_ids: vec![],
            projects: vec![],
            deleted_project_ids: vec![],
        };

        let (status, resp) = push(State(state), Json(req)).await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert!(resp.ok);
        assert!(resp.conflicts.is_empty());
    }

    #[tokio::test]
    async fn push_detects_conflicts() {
        let state = test_state();
        // Pre-insert a task with a newer timestamp
        state
            .db
            .upsert_task(
                "t1",
                r#"{"id":"t1","title":"server version"}"#,
                "2026-01-02T00:00:00Z",
                "dev0",
            )
            .unwrap();

        let req = PushRequest {
            device_id: "dev1".to_string(),
            tasks: vec![serde_json::json!({
                "id": "t1",
                "title": "client version",
                "updated_at": "2026-01-01T00:00:00Z",
            })],
            deleted_ids: vec![],
            projects: vec![],
            deleted_project_ids: vec![],
        };

        let (status, resp) = push(State(state), Json(req)).await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert!(!resp.ok);
        assert_eq!(resp.conflicts.len(), 1);
        assert_eq!(resp.conflicts[0].id, "t1");
        assert_eq!(resp.conflicts[0].server_updated_at, "2026-01-02T00:00:00Z");
    }

    #[tokio::test]
    async fn push_missing_task_id_returns_400() {
        let state = test_state();
        let req = PushRequest {
            device_id: "dev1".to_string(),
            tasks: vec![serde_json::json!({
                "title": "no id",
                "updated_at": "2026-01-01T00:00:00Z",
            })],
            deleted_ids: vec![],
            projects: vec![],
            deleted_project_ids: vec![],
        };

        let result = push(State(state), Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn push_handles_projects() {
        let state = test_state();
        let req = PushRequest {
            device_id: "dev1".to_string(),
            tasks: vec![],
            deleted_ids: vec![],
            projects: vec![serde_json::json!({
                "id": "p1",
                "name": "My Project",
                "updated_at": "2026-01-01T00:00:00Z",
            })],
            deleted_project_ids: vec![],
        };

        let (status, resp) = push(State(state), Json(req)).await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert!(resp.ok);
    }

    #[tokio::test]
    async fn pull_returns_changes() {
        let state = test_state();
        state
            .db
            .upsert_task(
                "t1",
                r#"{"id":"t1","title":"task1"}"#,
                "2026-01-01T00:00:00Z",
                "dev0",
            )
            .unwrap();

        let req = PullRequest {
            device_id: "dev1".to_string(),
            since: "2025-12-31T00:00:00Z".to_string(),
        };

        let (status, resp) = pull(State(state.clone()), Json(req))
            .await
            .unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(resp.tasks.len(), 1);
        assert_eq!(resp.tasks[0]["id"], "t1");
        assert!(!resp.server_time.is_empty());

        // Device should be recorded
        let info = state.db.get_status().unwrap();
        assert_eq!(info.devices.len(), 1);
        assert_eq!(info.devices[0].device_id, "dev1");
    }

    #[tokio::test]
    async fn pull_returns_deletions() {
        let state = test_state();
        state
            .db
            .upsert_task(
                "t1",
                r#"{"id":"t1"}"#,
                "2026-01-01T00:00:00Z",
                "dev0",
            )
            .unwrap();
        state
            .db
            .soft_delete_task("t1", "2026-01-02T00:00:00Z", "dev0")
            .unwrap();

        let req = PullRequest {
            device_id: "dev1".to_string(),
            since: "2025-12-31T00:00:00Z".to_string(),
        };

        let (status, resp) = pull(State(state), Json(req)).await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert!(resp.tasks.is_empty());
        assert_eq!(resp.deleted_ids.len(), 1);
        assert_eq!(resp.deleted_ids[0], "t1");
    }

    #[tokio::test]
    async fn status_returns_info() {
        let state = test_state();
        state
            .db
            .upsert_task(
                "t1",
                r#"{"id":"t1"}"#,
                "2026-01-01T00:00:00Z",
                "dev1",
            )
            .unwrap();
        state
            .db
            .record_device_sync("dev1", "2026-01-01T00:00:00Z")
            .unwrap();

        let (status, resp) = status(State(state)).await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(resp.total_tasks, 1);
        assert_eq!(resp.devices.len(), 1);
    }

    #[tokio::test]
    async fn push_soft_deletes() {
        let state = test_state();
        state
            .db
            .upsert_task(
                "t1",
                r#"{"id":"t1"}"#,
                "2026-01-01T00:00:00Z",
                "dev0",
            )
            .unwrap();

        let req = PushRequest {
            device_id: "dev1".to_string(),
            tasks: vec![],
            deleted_ids: vec!["t1".to_string()],
            projects: vec![],
            deleted_project_ids: vec![],
        };

        let (status, resp) = push(State(state.clone()), Json(req))
            .await
            .unwrap();
        assert_eq!(status, StatusCode::OK);
        assert!(resp.ok);

        let info = state.db.get_status().unwrap();
        assert_eq!(info.total_tasks, 0);
    }
}
