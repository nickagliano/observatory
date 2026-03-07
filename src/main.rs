mod alert;
mod dashboard;
mod db;
mod poller;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use axum::{Router, routing::get};
use rusqlite::Connection;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present
    dotenvy::dotenv().ok();

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9090);
    let txtme_url = std::env::var("TXTME_URL").ok();
    let interval_secs: u64 = std::env::var("POLL_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    // Open SQLite
    let db_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".epc/observatory.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&db_path)?;
    db::init(&conn)?;
    let db = Arc::new(Mutex::new(conn));

    // HTTP client for health checks + alerts
    let http = reqwest::Client::new();

    // Spawn poller background task
    {
        let db2 = Arc::clone(&db);
        let http2 = http.clone();
        tokio::spawn(async move {
            poller::run(db2, http2, txtme_url, interval_secs).await;
        });
    }

    // Axum router
    let app = Router::new()
        .route("/", get(dashboard::handler))
        .route("/health", get(|| async { "ok" }))
        .route("/api/services", get(api_services))
        .with_state(db);

    let addr = format!("{host}:{port}");
    println!("[observatory] listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn api_services(
    axum::extract::State(db): axum::extract::State<Arc<Mutex<Connection>>>,
) -> axum::Json<serde_json::Value> {
    let conn = db.lock().unwrap();
    let states = db::all_states(&conn).unwrap_or_default();
    let arr: Vec<_> = states
        .iter()
        .map(|s| {
            serde_json::json!({
                "service": s.service,
                "status": s.last_status,
                "last_checked": s.last_checked,
            })
        })
        .collect();
    axum::Json(serde_json::json!(arr))
}
