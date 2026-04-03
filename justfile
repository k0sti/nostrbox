# Nostrbox development commands

# Default: show available commands
default:
    @just --list

# First-time setup: generate identity and config
init:
    cargo run --bin nostrbox-init

# Build everything (server + web)
build:
    cargo build
    cd web && bun run build

# Build release
build-release:
    cargo build --release
    cd web && bun run build

# Run the server (dev mode)
run:
    cargo run --bin nostrbox-server

# Run with auto-reload (requires cargo-watch)
dev:
    cargo watch -x 'run --bin nostrbox-server'

# Run all tests
test:
    cargo test --workspace

# Check compilation without building
check:
    cargo check --workspace

# Build and run web dev server (with hot reload)
web-dev:
    cd web && bun dev

# Install web dependencies
web-install:
    cd web && bun install

# Format code
fmt:
    cargo fmt --all

# Lint
lint:
    cargo clippy --workspace
    cd web && bun run lint

# Build release, run tests, and restart the service
deploy:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== Testing ==="
    cargo test --workspace
    echo "=== Building release ==="
    cargo build --release
    cd web && bun run build
    cd ..
    echo "=== Restarting nostrbox service ==="
    sudo systemctl restart nostrbox
    sleep 1
    systemctl is-active --quiet nostrbox && echo "=== Deployed successfully ===" || { echo "=== Service failed to start ==="; sudo journalctl -u nostrbox -n 20 --no-pager; exit 1; }

# Build and restart without running tests
deploy-quick:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== Building release ==="
    cargo build --release
    cd web && bun run build
    cd ..
    echo "=== Restarting nostrbox service ==="
    sudo systemctl restart nostrbox
    sleep 1
    systemctl is-active --quiet nostrbox && echo "=== Deployed successfully ===" || { echo "=== Service failed to start ==="; sudo journalctl -u nostrbox -n 20 --no-pager; exit 1; }

# Show service status and recent logs
status:
    @systemctl status nostrbox --no-pager
    @echo ""
    @echo "=== Recent logs ==="
    @sudo journalctl -u nostrbox -n 20 --no-pager

# Follow service logs
logs:
    sudo journalctl -u nostrbox -f

# Rebuild NixOS configuration (dev profile)
nixos-rebuild:
    sudo nixos-rebuild switch --flake ./nixos#mac-mini-dev

# Clean build artifacts
clean:
    cargo clean
    rm -rf web/dist
