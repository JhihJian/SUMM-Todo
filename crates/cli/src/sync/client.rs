use crate::error::TodoError;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PushPayload {
    pub device_id: String,
    pub tasks: Vec<serde_json::Value>,
    pub deleted_ids: Vec<String>,
    pub projects: Vec<serde_json::Value>,
    pub deleted_project_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PushResponse {
    pub ok: bool,
    #[serde(default)]
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub client_updated_at: String,
    pub server_updated_at: String,
    pub server_data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct PullPayload {
    pub device_id: String,
    pub since: String,
}

#[derive(Debug, Deserialize)]
pub struct PullResponse {
    pub tasks: Vec<serde_json::Value>,
    pub deleted_ids: Vec<String>,
    pub projects: Vec<serde_json::Value>,
    pub deleted_project_ids: Vec<String>,
    pub server_time: String,
}

#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    pub total_tasks: i64,
    pub last_modified: Option<String>,
    pub devices: Vec<DeviceInfo>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_sync: String,
}

// ---------------------------------------------------------------------------
// Error response
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    ok: bool,
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct SyncClient {
    http: reqwest::blocking::Client,
    base_url: String,
    api_key: String,
}

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

impl SyncClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            http: reqwest::blocking::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    fn handle_error(&self, result: reqwest::Result<reqwest::blocking::Response>) -> Result<reqwest::blocking::Response, TodoError> {
        match result {
            Ok(resp) => {
                if resp.status().is_success() {
                    Ok(resp)
                } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
                    Err(TodoError::SyncAuthFailed)
                } else {
                    let status = resp.status();
                    match resp.json::<ErrorResponse>() {
                        Ok(err_resp) => Err(TodoError::SyncError(format!(
                            "{}: {}",
                            err_resp.error.code, err_resp.error.message
                        ))),
                        Err(_) => Err(TodoError::SyncError(format!("HTTP {}", status))),
                    }
                }
            }
            Err(e) if e.is_timeout() || e.is_connect() => {
                Err(TodoError::SyncServerUnreachable)
            }
            Err(e) => Err(TodoError::SyncError(e.to_string())),
        }
    }

    fn with_retry<F, T>(&self, f: F) -> Result<T, TodoError>
    where
        F: Fn() -> Result<T, TodoError>,
    {
        match f() {
            Ok(v) => Ok(v),
            Err(TodoError::SyncServerUnreachable) => {
                std::thread::sleep(std::time::Duration::from_secs(1));
                match f() {
                    Ok(v) => Ok(v),
                    Err(_) => Err(TodoError::SyncServerUnreachable),
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn push(&self, payload: &PushPayload) -> Result<PushResponse, TodoError> {
        self.with_retry(|| {
            let resp = self.http
                .post(self.url("/sync/push"))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(payload)
                .timeout(TIMEOUT)
                .send();
            let resp = self.handle_error(resp)?;
            resp.json::<PushResponse>()
                .map_err(|e| TodoError::SyncError(format!("parse error: {}", e)))
        })
    }

    pub fn pull(&self, payload: &PullPayload) -> Result<PullResponse, TodoError> {
        self.with_retry(|| {
            let resp = self.http
                .post(self.url("/sync/pull"))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(payload)
                .timeout(TIMEOUT)
                .send();
            let resp = self.handle_error(resp)?;
            resp.json::<PullResponse>()
                .map_err(|e| TodoError::SyncError(format!("parse error: {}", e)))
        })
    }

    pub fn status(&self) -> Result<StatusResponse, TodoError> {
        self.with_retry(|| {
            let resp = self.http
                .get(self.url("/sync/status"))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .timeout(TIMEOUT)
                .send();
            let resp = self.handle_error(resp)?;
            resp.json::<StatusResponse>()
                .map_err(|e| TodoError::SyncError(format!("parse error: {}", e)))
        })
    }
}
