mod config;
mod db;
mod handlers;
mod router;

use std::env;
use std::sync::Arc;

/// Shared application state passed to all handlers via axum State extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<db::SyncDb>,
    pub api_key: String,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let config = config::Config::parse(&args[1..]).unwrap_or_else(|e| {
        eprintln!("Configuration error: {e}");
        std::process::exit(1);
    });

    let db = db::SyncDb::open(&config.db_path).unwrap_or_else(|e| {
        eprintln!("Failed to open database at {}: {e}", config.db_path);
        std::process::exit(1);
    });

    let state = AppState {
        db: Arc::new(db),
        api_key: config.api_key,
    };

    let app = router::build_router(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.port));
    println!("SUMM sync server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
