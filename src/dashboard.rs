use std::sync::{Arc, Mutex};

use axum::{extract::State, response::Html};
use rusqlite::Connection;

use crate::db;

pub async fn handler(State(db): State<Arc<Mutex<Connection>>>) -> Html<String> {
    Html(render(&db))
}

pub fn render_log_page(service: &str, content: &str) -> String {
    let escaped = html_escape_log(content);
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{service} — logs</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      background: #0f0f1a;
      color: #c8c8e8;
      font-family: 'SF Mono', 'Menlo', monospace;
      font-size: 12px;
      padding: 16px;
    }}
    .header {{
      display: flex;
      align-items: center;
      gap: 12px;
      margin-bottom: 16px;
    }}
    a.back {{
      color: #6060a0;
      text-decoration: none;
      font-size: 12px;
    }}
    a.back:hover {{ color: #a0a0c0; }}
    h1 {{
      font-size: 14px;
      font-weight: 600;
      color: #a0a0c0;
      letter-spacing: 0.05em;
    }}
    pre {{
      white-space: pre;
      overflow-x: auto;
      line-height: 1.6;
      color: #c8c8e8;
    }}
  </style>
</head>
<body>
  <div class="header">
    <a class="back" href="/">← Observatory</a>
    <h1>{service}</h1>
  </div>
  <pre>{escaped}</pre>
</body>
</html>"#)
}

fn html_escape_log(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn status_pip(status: &str) -> String {
    format!("<span class=\"pip {status}\"></span>")
}

fn dot(status: &str) -> &'static str {
    // \u{FE0E} is the Unicode text variation selector — forces text rendering
    // instead of emoji on iOS, which would otherwise replace ● with a color emoji.
    match status {
        "running"  => "<span class=\"dot running\">\u{25CF}\u{FE0E}</span>",
        "degraded" => "<span class=\"dot degraded\">\u{25D0}\u{FE0E}</span>",
        _          => "<span class=\"dot stopped\">\u{25CB}\u{FE0E}</span>",
    }
}

fn render(db: &Arc<Mutex<Connection>>) -> String {
    let conn = db.lock().unwrap();

    let states = db::all_states(&conn).unwrap_or_default();

    let mut cards = String::new();
    for s in &states {
        let checks = db::recent_checks(&conn, &s.service, 40).unwrap_or_default();

        let latest_ms = checks.first().and_then(|c| c.response_ms);
        let ms_label = match latest_ms {
            Some(ms) => format!("{ms}ms"),
            None => "--".to_string(),
        };

        // Dots: checks are newest-first, we render oldest→newest (left→right)
        let mut dots = String::new();
        for check in checks.iter().rev() {
            dots.push_str(dot(&check.status));
        }

        let pip = status_pip(&s.last_status);
        let name = &s.service;
        let last_checked = &s.last_checked;
        let ci_badge = match &s.repo_url {
            Some(url) => format!(
                r#"<a href="{url}/actions" target="_blank" class="ci-link"><img src="{url}/actions/workflows/ci.yml/badge.svg" alt="CI" class="ci-badge" loading="lazy" onerror="this.closest('.ci-link').remove()"></a>"#
            ),
            None => String::new(),
        };

        cards.push_str(&format!(
            r#"<a href="/logs/{name}" class="card-link">
<div class="card">
  <div class="card-header">
    <span class="svc-name">{name}</span>
    <span class="badge">{pip}{status}</span>
  </div>
  <div class="card-meta">{ms_label} &nbsp;·&nbsp; {last_checked}</div>
  <div class="sparkline">{dots}</div>
  {ci_badge}
</div>
</a>
"#,
            status = s.last_status,
        ));
    }

    if cards.is_empty() {
        cards = r#"<div class="card empty">No services found in ~/.epc/services.toml</div>"#.to_string();
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta http-equiv="refresh" content="30">
<title>Observatory</title>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{
    background: #0f0f1a;
    color: #e0e0f0;
    font-family: 'SF Mono', 'Menlo', monospace;
    font-size: 14px;
    padding: 16px;
    padding-bottom: calc(64px + env(safe-area-inset-bottom));
    max-width: 480px;
    margin: 0 auto;
  }}
  h1 {{
    font-size: 18px;
    font-weight: 600;
    color: #a0a0c0;
    letter-spacing: 0.05em;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
  }}
  h1 svg {{
    opacity: 0.5;
  }}
  .hex-icon polygon {{
    stroke: #a0a0c0;
    stroke-width: 1.5;
    stroke-linejoin: round;
  }}
  .card {{
    background: #1a1a2e;
    border: 1px solid #2a2a4a;
    border-radius: 10px;
    padding: 14px;
    margin-bottom: 12px;
  }}
  .card.empty {{
    color: #606080;
    font-style: italic;
  }}
  .card-header {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }}
  .svc-name {{
    font-weight: 600;
    font-size: 15px;
    color: #c8c8f0;
  }}
  .badge {{
    font-size: 12px;
    color: #9090b0;
    display: flex;
    align-items: center;
    gap: 5px;
  }}
  .pip {{
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }}
  .pip.running  {{ background: #4caf50; box-shadow: 0 0 5px #4caf5055; }}
  .pip.degraded {{ background: #ff9800; box-shadow: 0 0 5px #ff980055; }}
  .pip.stopped  {{ background: #2a2a4a; border: 1px solid #444466; }}
  .card-meta {{
    font-size: 11px;
    color: #505070;
    margin-bottom: 8px;
  }}
  .sparkline {{
    font-size: 13px;
    letter-spacing: 1px;
    line-height: 1;
    white-space: nowrap;
    overflow: hidden;
  }}
  .dot.running  {{ color: #4caf50; }}
  .dot.degraded {{ color: #ff9800; }}
  .dot.stopped  {{ color: #444466; }}
  .ci-link {{ display: inline-block; margin-top: 8px; }}
  .ci-badge {{ height: 18px; border-radius: 3px; vertical-align: middle; }}
  a.card-link {{
    display: block;
    text-decoration: none;
    color: inherit;
    margin-bottom: 12px;
  }}
  a.card-link .card {{ margin-bottom: 0; }}
  a.card-link:hover .card {{ border-color: #4a4a7a; }}
  .bottom-bar {{
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 12px 20px;
    padding-bottom: calc(12px + env(safe-area-inset-bottom));
    background: rgba(15, 15, 26, 0.65);
    -webkit-backdrop-filter: blur(24px) saturate(180%);
    backdrop-filter: blur(24px) saturate(180%);
    border-top: 1px solid rgba(160, 160, 220, 0.1);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }}
  .bottom-bar .label {{
    font-size: 11px;
    color: #505070;
    letter-spacing: 0.04em;
  }}
  .bottom-bar .pulse {{
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #4caf50;
    box-shadow: 0 0 6px #4caf50;
    animation: pulse 2s infinite;
  }}
  @keyframes pulse {{
    0%, 100% {{ opacity: 1; }}
    50% {{ opacity: 0.3; }}
  }}
</style>
</head>
<body>
<h1>
  <svg class="hex-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
    <polygon points="8,1 14.5,4.5 14.5,11.5 8,15 1.5,11.5 1.5,4.5"/>
  </svg>
  Observatory
</h1>
{cards}
<div class="bottom-bar">
  <span class="label">Observatory &nbsp;·&nbsp; refreshes every 30s</span>
  <div class="pulse"></div>
</div>
</body>
</html>
"#
    )
}
