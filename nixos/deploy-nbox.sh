#!/usr/bin/env bash
# Deploy NostrBox NixOS config to nbox (mac-mini)
# Usage: ./deploy-nbox.sh [switch|test|build]
set -euo pipefail

NBOX="k0@10.10.241.148"
ACTION="${1:-switch}"

echo "=== Setting up FIPS keys on nbox ==="
ssh "$NBOX" 'sudo mkdir -p /etc/fips && sudo chmod 755 /etc/fips'

# Copy key files if they don't exist
ssh "$NBOX" 'test -f /etc/fips/fips.key' 2>/dev/null || {
  echo "Copying FIPS key files..."
  scp /tmp/fips-test-nbox/fips.key /tmp/fips-test-nbox/fips.pub "$NBOX":/tmp/
  ssh "$NBOX" 'sudo cp /tmp/fips.key /tmp/fips.pub /etc/fips/ && sudo chmod 600 /etc/fips/fips.key'
}

echo ""
echo "=== Building and deploying NixOS config ==="
cd "$(dirname "$0")"
nixos-rebuild "$ACTION" --flake .#mac-mini --target-host "$NBOX" --use-remote-sudo

echo ""
echo "=== Done! Check FIPS status ==="
ssh "$NBOX" 'sudo systemctl status fips --no-pager -l' || true
