use anyhow::Result;
use rusqlite::{Connection, params};

pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS health_checks (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            service     TEXT NOT NULL,
            checked_at  TEXT NOT NULL,
            status      TEXT NOT NULL,
            response_ms INTEGER,
            status_code INTEGER
        );
        CREATE TABLE IF NOT EXISTS service_state (
            service      TEXT PRIMARY KEY,
            last_status  TEXT NOT NULL,
            last_checked TEXT NOT NULL
        );",
    )?;
    Ok(())
}

pub fn insert_check(
    conn: &Connection,
    service: &str,
    checked_at: &str,
    status: &str,
    response_ms: Option<i64>,
    status_code: Option<u16>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO health_checks (service, checked_at, status, response_ms, status_code)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![service, checked_at, status, response_ms, status_code.map(|c| c as i64)],
    )?;
    Ok(())
}

/// Returns the last_status for a service, or None if not seen before.
pub fn get_last_status(conn: &Connection, service: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT last_status FROM service_state WHERE service = ?1",
    )?;
    let mut rows = stmt.query(params![service])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn set_last_status(conn: &Connection, service: &str, status: &str, checked_at: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO service_state (service, last_status, last_checked)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(service) DO UPDATE SET last_status = ?2, last_checked = ?3",
        params![service, status, checked_at],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub struct CheckRow {
    pub status: String,
    pub response_ms: Option<i64>,
    pub checked_at: String,
}

/// Returns the last `limit` checks for a service, newest first.
pub fn recent_checks(conn: &Connection, service: &str, limit: usize) -> Result<Vec<CheckRow>> {
    let mut stmt = conn.prepare(
        "SELECT status, response_ms, checked_at
         FROM health_checks
         WHERE service = ?1
         ORDER BY id DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![service, limit as i64], |row| {
        Ok(CheckRow {
            status: row.get(0)?,
            response_ms: row.get(1)?,
            checked_at: row.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Snapshot of current state for all services.
pub struct ServiceSnapshot {
    pub service: String,
    pub last_status: String,
    pub last_checked: String,
}

pub fn all_states(conn: &Connection) -> Result<Vec<ServiceSnapshot>> {
    let mut stmt = conn.prepare(
        "SELECT service, last_status, last_checked FROM service_state ORDER BY service",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ServiceSnapshot {
            service: row.get(0)?,
            last_status: row.get(1)?,
            last_checked: row.get(2)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
