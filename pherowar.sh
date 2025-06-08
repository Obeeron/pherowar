#!/bin/bash
# Top-level run script for PheroWar

set -e

if [[ ! -f ./Application/bin/pherowar ]]; then
  echo "error: ./bin/pherowar not found. First run ./build.sh"
  exit 1
fi

if [[ ! -f ./Application/bin/player ]]; then
  echo "error: ./bin/player not found. First run ./build.sh"
  exit 1
fi

if ! podman image exists pherowar-player:latest; then
  echo "error: podman image 'pherowar-player:latest' not found. First run ./build.sh"
  exit 1
fi

./Application/bin/pherowar $@
