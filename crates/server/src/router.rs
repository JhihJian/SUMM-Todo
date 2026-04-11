use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;

use crate::handlers;
use crate::AppState;

/// Auth middleware that checks `Authorization: Bearer <key>` against config.
async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: middleware::Next,
) -> Result<Response, (StatusCode, axum::Json<serde_json::Value>)> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let key = &header[7..];
            if key == state.api_key {
                Ok(next.run(req).await)
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({
                        "error": "E_UNAUTHORIZED",
                        "message": "Invalid API key",
                    })),
                ))
            }
        }
        _ => Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "E_UNAUTHORIZED",
                "message": "Missing or invalid Authorization header",
            })),
        )),
    }
}

/// Build the axum Router with sync routes and auth middleware.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/sync/push", post(handlers::push))
        .route("/api/v1/sync/pull", post(handlers::pull))
        .route("/api/v1/sync/status", get(handlers::status))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::db::SyncDb;
    use crate::router::build_router;
    use crate::AppState;

    fn test_app(api_key: &str) -> Router {
        let db = SyncDb::open_in_memory().unwrap();
        let state = AppState {
            db: Arc::new(db),
            api_key: api_key.to_string(),
        };
        build_router(state)
    }

    async fn send_request(
        app: &Router,
        method: &str,
        uri: &str,
        body: Option<String>,
        auth: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(key) = auth {
            builder = builder.header("Authorization", format!("Bearer {}", key));
        }
        let req = match body {
            Some(b) => builder
                .header("Content-Type", "application/json")
                .body(Body::from(b))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };

        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body_bytes).unwrap_or(Value::Null);
        (status, body)
    }

    #[tokio::test]
    async fn reject_no_auth() {
        let app = test_app("secret");
        let (status, body) = send_request(&app, "GET", "/api/v1/sync/status", None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "E_UNAUTHORIZED");
    }

    #[tokio::test]
    async fn reject_wrong_auth() {
        let app = test_app("secret");
        let (status, body) =
            send_request(&app, "GET", "/api/v1/sync/status", None, Some("wrong")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "E_UNAUTHORIZED");
    }

    #[tokio::test]
    async fn accept_correct_auth() {
        let app = test_app("secret");
        let (status, body) =
            send_request(&app, "GET", "/api/v1/sync/status", None, Some("secret")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["total_tasks"], 0);
    }

    #[tokio::test]
    async fn push_and_pull_roundtrip() {
        let app = test_app("mykey");

        // Push a task
        let push_body = serde_json::json!({
            "device_id": "dev1",
            "tasks": [{
                "id": "t1",
                "title": "Hello",
                "updated_at": "2026-01-01T00:00:00Z",
            }],
            "deleted_ids": [],
            "projects": [],
            "deleted_project_ids": [],
        })
        .to_string();

        let (status, body) = send_request(
            &app,
            "POST",
            "/api/v1/sync/push",
            Some(push_body),
            Some("mykey"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], true);

        // Pull
        let pull_body = serde_json::json!({
            "device_id": "dev2",
            "since": "2025-12-31T00:00:00Z",
        })
        .to_string();

        let (status, body) = send_request(
            &app,
            "POST",
            "/api/v1/sync/pull",
            Some(pull_body),
            Some("mykey"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["tasks"].as_array().unwrap().len(), 1);
        assert_eq!(body["tasks"][0]["id"], "t1");
    }
}
