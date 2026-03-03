# observatory — Customization Guide

observatory is a self-hosted observability stack for EPS services. It ships a working
OTel Collector → Prometheus + Grafana Tempo → Grafana pipeline out of the box. Point
your services at `localhost:4317` (OTLP gRPC) and open Grafana at `localhost:3000`.

The three ports — DASHBOARDS, SCRAPE_TARGETS, and RETENTION — cover the common things
you'll want to change.

## Ports

### `DASHBOARDS`

**What it does:** Controls which Grafana dashboards are loaded automatically on startup.

**Default:** `dashboards/home.json` — host CPU/RAM/disk panels + a fleet overview row +
epw panels that show "no data" until epw is connected. EPS packages that emit metrics
contribute additional dashboard JSON files to this port.

**How to customize:** Drop any Grafana dashboard JSON file into `./dashboards/`. Grafana
polls this directory every 30 seconds and loads new files automatically (no restart needed).

To install a dashboard from an EPS package:
```sh
# Example: installing the epw dashboard
cp /path/to/extremely_personal_whisper/contrib/observatory/epw-dashboard.json ./dashboards/
```

To create your own, export a dashboard from Grafana UI: Dashboard → Share → Export →
Save to file → copy the file into `./dashboards/`.

Sub-directories are supported and become Grafana folders:
```
dashboards/
  epw/
    epw-dashboard.json      → Grafana folder "epw"
  infra/
    node-exporter.json      → Grafana folder "infra"
```

---

### `SCRAPE_TARGETS`

**What it does:** Controls which services Prometheus scrapes directly via `/metrics`
endpoints. Distinct from OTLP — services that push via the OTel Collector do not need
a scrape target.

**Default:** Prometheus self-monitors (`localhost:9090`) and the OTel Collector
(`otel-collector:8888`).

**How to customize:** Edit `config/prometheus.yml`, adding entries under `scrape_configs`:

```yaml
scrape_configs:
  # existing entries...

  - job_name: "my_service"
    static_configs:
      - targets: ["host.docker.internal:8080"]   # use host.docker.internal to reach host
```

For services on other machines, use their IP or Tailscale hostname.

---

### `RETENTION`

**What it does:** Controls how long metrics and traces are stored before being deleted.

**Default:**
- Metrics (Prometheus): 30 days
- Traces (Grafana Tempo): 14 days

**How to customize:**

Metrics — edit `docker-compose.yml`, Prometheus `command:` section:
```yaml
- "--storage.tsdb.retention.time=90d"   # change to desired duration
```

Traces — edit `config/tempo.yml`:
```yaml
compactor:
  compaction:
    block_retention: 720h   # 720h = 30 days
```

Restart after changing: `./run.sh restart`

---

## Getting Started

1. Clone: `git clone https://github.com/nickagliano/observatory`
2. Ensure Docker Desktop is running
3. `./run.sh start`
4. Open Grafana: http://localhost:3000

To receive OTLP data from a service, point it at:
- gRPC: `localhost:4317`
- HTTP: `localhost:4318`

## Common Customizations

### Installing an EPS dashboard

```sh
# Copy the dashboard JSON from the EPS package's contrib/ directory
cp ~/Documents/personal-projects/extremely_personal_whisper/contrib/observatory/epw-dashboard.json \
   ./dashboards/

# Grafana loads it within 30 seconds — no restart needed
```

### Watching logs for a specific service

```sh
./run.sh logs otel-collector
./run.sh logs tempo
./run.sh logs prometheus
./run.sh logs grafana
```

### Wiping all data and starting fresh

```sh
./run.sh reset   # prompts for confirmation, then removes Docker volumes
./run.sh start
```

---

## EPS Contribution Convention

This section is for EPS authors who want their package to work with observatory.

### How an EPS plugs into observatory

An EPS contributes to observatory through two mechanisms:

1. **OTLP metrics/traces** — the EPS emits OTLP at runtime, pointed at `localhost:4317`.
   The operator configures the endpoint (never hardcoded in the EPS). Metrics appear in
   Prometheus automatically; traces appear in Grafana Tempo automatically.

2. **Dashboard JSON** — the EPS ships a pre-built Grafana dashboard at
   `contrib/observatory/<package-name>.json` in its own repo. The operator copies it
   into observatory's `./dashboards/` directory.

### Dashboard file convention

```
your_eps_repo/
  contrib/
    observatory/
      your_package.json    ← Grafana dashboard JSON, exported from Grafana UI
```

Rules for the dashboard JSON:
- Use `"uid": "eps-<package-name>"` to avoid collisions with other EPS dashboards.
- Use the `${DS_PROMETHEUS}` and `${DS_TEMPO}` datasource template variables (not
  hardcoded UIDs) so the dashboard works in any Grafana instance.
- Only query metric names your EPS actually emits. Don't reference `epw_*` metrics
  from a non-epw EPS.
- Include `"tags": ["eps", "<package-name>"]` for discoverability in Grafana search.

### Minimal OTLP config pattern (Rust / epw-otel style)

```toml
# EPS config file — operator sets this, EPS never hardcodes it
[telemetry]
endpoint = "http://localhost:4317"   # points at observatory
```

The EPS reads the endpoint at startup and passes it to `opentelemetry-otlp`. If the
config key is absent, the EPS emits nothing (opt-in by default, ADR-007 style).

### Currently contributing EPS packages

| EPS | Metrics prefix | Dashboard |
|-----|---------------|-----------|
| `extremely_personal_whisper` | `epw.*` | `contrib/observatory/epw-dashboard.json` (planned) |

To add your EPS to this table, open a PR to `observatory` with an update to this file.
