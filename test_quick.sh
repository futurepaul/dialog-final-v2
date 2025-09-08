#!/bin/bash

# Quick test script for Dialog CLI
# Simple test to verify basic functionality

set -e

# Configuration
TEST_NSEC="nsec1ufnus6pju578ste3v90xd5m2decpuzpql2295m3sknqcjzyys9ls0qlc85"
export DIALOG_NSEC=$TEST_NSEC

echo "=== Quick Dialog CLI Test ==="
echo ""

# Build if needed
echo "Building CLI..."
cd dialog_cli && cargo build --release && cd ..

# Start relay
echo "Starting relay..."
pkill -f "nak.*serve.*10548" 2>/dev/null || true
nak serve --port 10548 > /tmp/nak-test.log 2>&1 &
NAK_PID=$!
sleep 2

# Cleanup on exit
trap "kill $NAK_PID 2>/dev/null || true" EXIT

# Create a timestamped note
TIMESTAMP=$(date +"%Y-%m-%d %H:%M:%S")
NOTE_TEXT="Test note created at $TIMESTAMP #quicktest"

echo "Creating test note..."
./dialog_cli/target/release/dialog_cli create "$NOTE_TEXT"

echo ""
echo "Listing notes..."
./dialog_cli/target/release/dialog_cli list --limit 5

echo ""
echo "Listing notes with #quicktest tag..."
./dialog_cli/target/release/dialog_cli list --tag quicktest --limit 5

echo ""
echo "✓ Quick test complete!"