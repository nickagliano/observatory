#!/usr/bin/env bash
set -euo pipefail

COMMAND="${1:-help}"

require_docker() {
  if ! docker info &>/dev/null; then
    echo "Docker daemon is not running. Attempting to start Docker Desktop..."
    open -a Docker 2>/dev/null || true
    echo "Waiting for Docker to be ready..."
    for i in $(seq 1 30); do
      if docker info &>/dev/null; then
        echo "Docker is ready."
        return
      fi
      sleep 2
    done
    echo "Error: Docker did not start in time. Open Docker Desktop manually and retry."
    exit 1
  fi
}

case "$COMMAND" in
  start)
    require_docker
    echo "Starting observatory..."
    docker compose up -d
    echo ""
    echo "  Grafana:    http://localhost:9030"
    echo "  Prometheus: http://localhost:9090"
    echo "  OTLP gRPC:  localhost:4317"
    echo "  OTLP HTTP:  localhost:4318"
    ;;
  stop)
    require_docker
    docker compose down
    ;;
  restart)
    require_docker
    docker compose restart
    ;;
  logs)
    require_docker
    docker compose logs -f "${2:-}"
    ;;
  status)
    require_docker
    docker compose ps
    ;;
  reset)
    echo "WARNING: This will delete all stored metrics and traces."
    read -r -p "Continue? [y/N] " confirm
    if [[ "$confirm" =~ ^[Yy]$ ]]; then
      docker compose down -v
      echo "Data volumes removed."
    fi
    ;;
  help|*)
    cat <<EOF
Usage: ./run.sh <command>

Commands:
  start       Start all services (detached)
  stop        Stop all services
  restart     Restart all services
  logs [svc]  Tail logs (optionally for one service)
  status      Show running containers
  reset       Stop and delete all stored data (irreversible)
EOF
    ;;
esac
