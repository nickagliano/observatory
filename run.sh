#!/usr/bin/env bash
set -euo pipefail

COMMAND="${1:-help}"

case "$COMMAND" in
  start)
    echo "Starting observatory..."
    docker compose up -d
    echo ""
    echo "  Grafana:    http://localhost:3000"
    echo "  Prometheus: http://localhost:9090"
    echo "  OTLP gRPC:  localhost:4317"
    echo "  OTLP HTTP:  localhost:4318"
    ;;
  stop)
    docker compose down
    ;;
  restart)
    docker compose restart
    ;;
  logs)
    docker compose logs -f "${2:-}"
    ;;
  status)
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
