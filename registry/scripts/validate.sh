#!/usr/bin/env bash
# Validate a community plugin submission
set -euo pipefail

PLUGIN_DIR="${1:?Usage: validate.sh <plugin-dir>}"

echo "Validating plugin at: $PLUGIN_DIR"

# Check required files
for f in Cargo.toml jovial-plugin.yaml src/lib.rs; do
    if [ ! -f "$PLUGIN_DIR/$f" ]; then
        echo "ERROR: Missing required file: $f"
        exit 1
    fi
done

# Build the plugin
cargo build --manifest-path "$PLUGIN_DIR/Cargo.toml"

echo "Validation passed!"
