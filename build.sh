#!/bin/bash
# Top-level build script for PheroWar
set -e

# Define directory and path variables
APP_DIR="Application"

# Output directories are relative to the script's execution path (project root)
OUTPUT_BIN_DIR="$APP_DIR/bin"
OUTPUT_PLAYERS_DIR="./players"

# Symlink sources are also relative to the project root
SYMLINK_MAPS_SOURCE="$APP_DIR/maps"
SYMLINK_PLAYERS_SOURCE="$APP_DIR/players"

# Build the pherowar build image using the Dockerfile.build
podman build -t pherowar-build:latest -f "$APP_DIR/Dockerfile.build" "$APP_DIR"

# Create a container from the pherowar-build image
container_id=$(podman create pherowar-build:latest)
if [[ -z "$container_id" ]]; then
    echo "Failed to create container from pherowar-build:latest image."
    exit 1
fi

# Recover the output binaries from the container
mkdir -p "$OUTPUT_BIN_DIR"
podman cp "$container_id:/out/bin/." "$OUTPUT_BIN_DIR/"

# User the player wrapper binary to build the player image
podman build -t pherowar-player:latest -f "$APP_DIR/Dockerfile.player" "$APP_DIR"


# Build the dummy example brain
make -C dummy-brain
mkdir -p ./players
cp dummy-brain/brain.so ./players/dummy_brain.so

echo ""
echo -e "\033[1mBuild complete.\033[0m You can now run the game using the \033[32mpherowar.sh\033[0m script."
