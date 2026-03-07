use std::{
    collections::HashMap,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use serde::Deserialize;
use tokio::time;

use crate::{alert, db};

// ── EPC state file structs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ServiceEntry {
    dir: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct ServicesFile {
    #[serde(default)]
    services: HashMap<String, ServiceEntry>,
}

// ── eps.toml minimal parser ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct EpsPackage {
    repository: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EpsService {
    health_check: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EpsManifest {
    package: Option<EpsPackage>,
    service: Option<EpsService>,
}

/// Returns (health_check, repo_url) from eps.toml.
fn read_eps_info(dir: &str) -> (Option<String>, Option<String>) {
    let path = PathBuf::from(dir).join("eps.toml");
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (None, None),
    };
    let manifest: EpsManifest = match toml::from_str(&content) {
        Ok(m) => m,
        Err(_) => return (None, None),
    };
    let health_check = manifest.service.and_then(|s| s.health_check);
    let repo_url = manifest.package.and_then(|p| p.repository);
    (health_check, repo_url)
}

// ── Port-listening check (mirrors EPC's approach) ─────────────────────────────

fn is_port_listening(port: u16) -> bool {
    let out = Command::new("lsof")
        .args(["-t", "-i", &format!(":{port}"), "-sTCP:LISTEN"])
        .output();
    match out {
        Ok(o) => !o.stdout.is_empty(),
        Err(_) => false,
    }
}

// ── Tailscale IP ──────────────────────────────────────────────────────────────

fn tailscale_ip() -> String {
    let out = Command::new("tailscale")
        .args(["ip", "-4"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    out.trim().to_string()
}

// ── EPC state file path ───────────────────────────────────────────────────────

fn services_toml_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".epc/services.toml")
}

// ── Main poll loop ────────────────────────────────────────────────────────────

pub async fn run(
    db: Arc<Mutex<Connection>>,
    http: reqwest::Client,
    txtme_url: Option<String>,
    interval_secs: u64,
) {
    let mut ticker = time::interval(Duration::from_secs(interval_secs));
    loop {
        ticker.tick().await;
        if let Err(e) = poll_once(&db, &http, txtme_url.as_deref()).await {
            eprintln!("[observatory] poll error: {e}");
        }
    }
}

async fn poll_once(
    db: &Arc<Mutex<Connection>>,
    http: &reqwest::Client,
    txtme_url: Option<&str>,
) -> Result<()> {
    let content = std::fs::read_to_string(services_toml_path())?;
    let file: ServicesFile = toml::from_str(&content)?;
    let ts_ip = tailscale_ip();

    for (name, entry) in &file.services {
        let (status, response_ms, status_code, repo_url) =
            check_service(http, name, entry, &ts_ip).await;

        let now = Utc::now().to_rfc3339();

        let prev = {
            let conn = db.lock().unwrap();
            db::get_last_status(&conn, name).unwrap_or(None)
        };

        {
            let conn = db.lock().unwrap();
            db::insert_check(&conn, name, &now, &status, response_ms, status_code).ok();
            db::set_last_status(&conn, name, &status, &now, repo_url.as_deref()).ok();
        }

        // Alert on transition
        if let Some(prev_status) = prev {
            if prev_status != status {
                if let Some(url) = txtme_url {
                    let msg = match status.as_str() {
                        "running" => format!("[Observatory] {name} recovered"),
                        "degraded" => format!("[Observatory] {name} is DEGRADED"),
                        _ => format!("[Observatory] {name} is DOWN"),
                    };
                    if let Err(e) = alert::send(http, url, &msg).await {
                        eprintln!("[observatory] alert failed for {name}: {e}");
                    }
                }
            }
        }
    }

    Ok(())
}

async fn check_service(
    http: &reqwest::Client,
    name: &str,
    entry: &ServiceEntry,
    ts_ip: &str,
) -> (String, Option<i64>, Option<u16>, Option<String>) {
    let (health_check, repo_url) = read_eps_info(&entry.dir);

    if !is_port_listening(entry.port) {
        return ("stopped".into(), None, None, repo_url);
    }

    if health_check.is_none() {
        return ("running".into(), None, None, repo_url);
    }

    let url = format!("http://{}:{}/health", ts_ip, entry.port);
    let start = std::time::Instant::now();
    match http
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) => {
            let ms = start.elapsed().as_millis() as i64;
            let code = resp.status().as_u16();
            if resp.status().is_success() {
                ("running".into(), Some(ms), Some(code), repo_url)
            } else {
                eprintln!("[observatory] {name} health returned {code}");
                ("degraded".into(), Some(ms), Some(code), repo_url)
            }
        }
        Err(e) => {
            let ms = start.elapsed().as_millis() as i64;
            eprintln!("[observatory] {name} health check failed: {e}");
            ("degraded".into(), Some(ms), None, repo_url)
        }
    }
}
