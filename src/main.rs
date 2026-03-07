mod alert;
mod dashboard;
mod db;
mod poller;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use axum::{Router, routing::get, http::{header, HeaderMap, StatusCode}, response::IntoResponse};
use tower_http::trace::TraceLayer;
use rusqlite::Connection;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
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
        .route("/logs/:service", get(logs_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(db);

    let addr = format!("{host}:{port}");
    println!("[observatory] listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn logs_handler(
    axum::extract::Path(service): axum::extract::Path<String>,
) -> impl IntoResponse {
    // Sanitize: prevent path traversal
    if !service.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return (StatusCode::BAD_REQUEST, HeaderMap::new(), "invalid service name".to_string())
            .into_response();
    }
    let log_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".epc/logs")
        .join(format!("{service}.log"));

    let content = std::fs::read_to_string(&log_path)
        .unwrap_or_else(|_| format!("No log file found at {}", log_path.display()));

    let html = dashboard::render_log_page(&service, &content);
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());
    (StatusCode::OK, headers, html).into_response()
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
