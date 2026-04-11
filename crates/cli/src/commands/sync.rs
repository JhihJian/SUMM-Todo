use chrono::Utc;
use crate::cli::{SyncArgs, SyncCommand};
use crate::db::Database;
use crate::error::TodoError;
use crate::output::Output;
use crate::sync::client::{PushPayload, PullPayload, SyncClient};
use crate::sync::tracker::SyncTracker;

pub fn execute(db: &Database, args: SyncArgs, _output: &Output) -> Result<String, TodoError> {
    match args.command {
        None => full_sync(db),
        Some(SyncCommand::SyncInit(a)) => sync_init(db, &a),
        Some(SyncCommand::SyncPush(_)) => sync_push(db),
        Some(SyncCommand::SyncPull(_)) => sync_pull(db),
        Some(SyncCommand::SyncStatus(_)) => sync_status(db),
    }
}

fn get_sync_config(db: &Database) -> Result<(String, String, String), TodoError> {
    let server_url = db.get_sync_config("server_url")?
        .ok_or_else(|| TodoError::SyncError("Not initialized. Run: todo sync init --server <url> --key <key>".into()))?;
    let api_key = db.get_sync_config("api_key")?
        .ok_or_else(|| TodoError::SyncError("Not initialized. Run: todo sync init --server <url> --key <key>".into()))?;
    let device_id = db.get_sync_config("device_id")?
        .ok_or_else(|| TodoError::SyncError("Not initialized. Run: todo sync init --server <url> --key <key>".into()))?;
    Ok((server_url, api_key, device_id))
}

fn sync_init(db: &Database, args: &crate::cli::SyncInitArgs) -> Result<String, TodoError> {
    // Generate or reuse device_id
    let device_id = match db.get_sync_config("device_id")? {
        Some(id) => id,
        None => {
            let uuid = uuid::Uuid::now_v7();
            let id = uuid.simple().to_string()[..8].to_string();
            db.set_sync_config("device_id", &id)?;
            id
        }
    };

    // Save server_url + api_key
    db.set_sync_config("server_url", &args.server)?;
    db.set_sync_config("api_key", &args.key)?;

    // Validate connectivity
    let client = SyncClient::new(&args.server, &args.key);
    let status = client.status()?;

    // Initial sync: pull first, then push
    let _ = sync_pull(db);
    let _ = sync_push(db);

    Ok(format!("Sync initialized. Device: {}. Server has {} tasks.", device_id, status.total_tasks))
}

fn full_sync(db: &Database) -> Result<String, TodoError> {
    sync_pull(db)?;
    sync_push(db)?;
    Ok("Sync complete.".to_string())
}

fn sync_push(db: &Database) -> Result<String, TodoError> {
    let (server_url, api_key, device_id) = get_sync_config(db)?;

    let tracker = SyncTracker::new(db);
    let (tasks, deleted_ids, projects, deleted_project_ids) = tracker.get_pending()?;

    let client = SyncClient::new(&server_url, &api_key);
    let payload = PushPayload {
        device_id,
        tasks: tasks.iter().map(|t| serde_json::to_value(t).unwrap()).collect(),
        deleted_ids,
        projects: projects.iter().map(|p| serde_json::to_value(p).unwrap()).collect(),
        deleted_project_ids,
    };

    let response = client.push(&payload)?;
    tracker.clear()?;

    // Update last_sync_at
    db.set_sync_config("last_sync_at", &Utc::now().to_rfc3339())?;

    if response.conflicts.is_empty() {
        Ok(format!("Pushed {} tasks, {} projects.", payload.tasks.len(), payload.projects.len()))
    } else {
        Ok(format!("Pushed {} tasks, {} projects. {} conflicts.",
            payload.tasks.len(), payload.projects.len(), response.conflicts.len()))
    }
}

fn sync_pull(db: &Database) -> Result<String, TodoError> {
    let (server_url, api_key, device_id) = get_sync_config(db)?;

    let since = db.get_sync_config("last_sync_at")?
        .unwrap_or_default();

    let client = SyncClient::new(&server_url, &api_key);
    let payload = PullPayload {
        device_id,
        since,
    };

    let response = client.pull(&payload)?;

    // Import pulled tasks
    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut deleted = 0u32;

    for task_value in &response.tasks {
        match serde_json::from_value::<crate::task::Task>(task_value.clone()) {
            Ok(task) => {
                // Skip if local version is newer
                if let Some(ref existing) = db.get_task(&task.id)? {
                    if let (Some(local), Some(remote)) = (existing.updated_at, task.updated_at) {
                        if local > remote {
                            skipped += 1;
                            continue;
                        }
                    }
                }
                // Upsert: try update first, then insert
                if db.get_task(&task.id)?.is_some() {
                    db.update_task(&task)?;
                } else {
                    db.insert_task(&task)?;
                }
                imported += 1;
            }
            Err(_) => skipped += 1,
        }
    }

    // Delete locally
    for id in &response.deleted_ids {
        if let Some(task) = db.get_task(id)? {
            // Only delete if local updated_at <= server deletion time
            // (we don't have per-deletion timestamps, so just delete)
            let _ = task;
            db.conn.execute("DELETE FROM tasks WHERE id = ?1", rusqlite::params![id])?;
            deleted += 1;
        }
    }

    // Import pulled projects
    let mut imported_projects = 0u32;
    for proj_value in &response.projects {
        match serde_json::from_value::<crate::task::Project>(proj_value.clone()) {
            Ok(project) => {
                if db.get_project(&project.id)?.is_some() {
                    db.update_project(&project)?;
                } else {
                    db.insert_project(&project)?;
                }
                imported_projects += 1;
            }
            Err(_) => {}
        }
    }

    for id in &response.deleted_project_ids {
        let _ = db.conn.execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![id]);
    }

    // Update last_sync_at
    db.set_sync_config("last_sync_at", &response.server_time)?;

    Ok(format!("Pulled {} tasks ({} deleted), {} projects.", imported, deleted, imported_projects))
}

fn sync_status(db: &Database) -> Result<String, TodoError> {
    let (server_url, api_key, _device_id) = get_sync_config(db)?;

    let client = SyncClient::new(&server_url, &api_key);
    let status = client.status()?;

    let mut lines = Vec::new();
    lines.push(format!("Server tasks: {}", status.total_tasks));
    if let Some(last) = status.last_modified {
        lines.push(format!("Last modified: {}", last));
    }
    if !status.devices.is_empty() {
        lines.push("Devices:".to_string());
        for d in &status.devices {
            lines.push(format!("  {} (last sync: {})", d.device_id, d.last_sync));
        }
    }

    // Local status
    if let Some(last_sync) = db.get_sync_config("last_sync_at")? {
        lines.push(format!("Local last sync: {}", last_sync));
    }

    Ok(lines.join("\n"))
}
