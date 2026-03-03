# observatory

Self-hosted observability for your EPS fleet. One command starts the full stack:

```sh
./run.sh start
```

Then open **http://localhost:3000** — Grafana, with host metrics and any EPS dashboards
already loaded.

## What's inside

```
OTel Collector  :4317 (gRPC) / :4318 (HTTP)   ← services send OTLP here
      │
      ├── metrics ──► Prometheus  :9090
      └── traces  ──► Grafana Tempo :3200
                              │
                         Grafana :3000   ← you look here
```

Node Exporter runs alongside and gives you CPU, RAM, disk, and network for the host
machine — useful for every EPS without any configuration.

## Connecting an EPS service

Any EPS that emits OTLP points at `localhost:4317`. That's it.

```toml
# Example: epw config
[telemetry]
endpoint = "http://localhost:4317"
```

To add a Grafana dashboard for your EPS, drop a JSON file into `./dashboards/`. Grafana
picks it up within 30 seconds. See **CUSTOMIZE.md** for the full contribution convention.

## Commands

| Command | What it does |
|---------|-------------|
| `./run.sh start` | Start all services (detached) |
| `./run.sh stop` | Stop all services |
| `./run.sh logs [service]` | Tail logs |
| `./run.sh status` | Show running containers |
| `./run.sh reset` | Wipe all stored data (asks first) |

## EPS dashboard contributions

EPS packages that want a pre-built dashboard ship a JSON file at
`contrib/observatory/<package-name>.json`. To install it:

```sh
cp /path/to/my_eps/contrib/observatory/my_eps.json ./dashboards/
```

## Ports (what to customize)

See `CUSTOMIZE.md` for the three extension points:
- **DASHBOARDS** — drop dashboard JSON files here
- **SCRAPE_TARGETS** — add Prometheus scrape targets for services that expose `/metrics`
- **RETENTION** — how long to keep metrics and traces
