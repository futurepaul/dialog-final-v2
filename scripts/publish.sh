#!/usr/bin/env bash
set -euo pipefail

echo "Publishing dialog_final_v2_lib first..."
cargo publish -p dialog_final_v2_lib

echo "Waiting for crate availability..."
sleep 30

echo "Publishing dialog_final_v2_cli..."
cargo publish -p dialog_final_v2_cli

echo "Done. Users can install via: cargo install dialog_final_v2_cli"

