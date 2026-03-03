#!/usr/bin/env bash
set -euo pipefail

# Verify Docker is running
if ! docker info &>/dev/null; then
  echo "Error: Docker is not running. Start Docker Desktop and retry."
  exit 1
fi

# Pull images upfront so first `run.sh start` is fast
docker compose pull

chmod +x run.sh

echo "observatory installed. Run ./run.sh start to launch."
