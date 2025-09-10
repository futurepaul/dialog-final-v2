#!/bin/bash

# Test watch mode functionality

set -e

if [ -z "$DIALOG_NSEC" ]; then
  echo "DIALOG_NSEC not set. Export your test nsec before running."
  exit 1
fi

echo "=== Testing Watch Mode ==="
echo ""

# Kill any existing relay
pkill -f "nak.*serve.*10548" 2>/dev/null || true
sleep 1

# Start relay
echo "Starting relay..."
nak serve --port 10548 > /tmp/nak-watch.log 2>&1 &
NAK_PID=$!
sleep 2

# Cleanup on exit
trap "kill $NAK_PID 2>/dev/null || true" EXIT

# Start watch mode in background
echo "Starting watch mode..."
./target/release/dialog_cli list --watch > /tmp/watch-output.log 2>&1 &
WATCH_PID=$!
sleep 2

# Create some notes
echo "Creating notes while watching..."
./target/release/dialog_cli create "First watched note #watch #test1"
sleep 1
./target/release/dialog_cli create "Second watched note #watch #test2"
sleep 1
./target/release/dialog_cli create "Third note without watch tag #other"
sleep 2

# Kill watch mode
kill $WATCH_PID 2>/dev/null || true

# Check output
echo ""
echo "=== Watch Mode Output ==="
cat /tmp/watch-output.log

echo ""
echo "=== Testing Tag-Filtered Watch ==="
echo ""

# Test with tag filter
./target/release/dialog_cli list --watch --tag watch > /tmp/watch-tag.log 2>&1 &
WATCH_TAG_PID=$!
sleep 2

# Create more notes
./target/release/dialog_cli create "Note with watch tag #watch #filtered"
sleep 1
./target/release/dialog_cli create "Note without watch tag #other #skip"
sleep 2

# Kill watch mode
kill $WATCH_TAG_PID 2>/dev/null || true

echo "=== Tag-Filtered Watch Output ==="
cat /tmp/watch-tag.log

echo ""
echo "✓ Watch test complete!"
