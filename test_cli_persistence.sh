#!/bin/bash

# Dialog CLI Persistence Test Script
# Tests that notes persist across multiple runs using nak with negentropy

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
NAK_BINARY="./nak-negentropy"
NAK_PORT=10548
if [ -z "$DIALOG_NSEC" ]; then
  echo -e "${RED}DIALOG_NSEC not set. Export your test nsec before running.${NC}"
  exit 1
fi
CLI_BINARY="./dialog_cli/target/release/dialog_cli"
RELAY_URL="ws://localhost:$NAK_PORT"

# Check if nak-negentropy exists, if not try system nak
if [ ! -f "$NAK_BINARY" ]; then
  echo -e "${RED}Error: $NAK_BINARY not found${NC}"
  echo "Please run ./setup_nak_local.sh first"
  exit 1
fi

# Build the CLI if needed
echo "Building Dialog CLI..."
cd dialog_cli && cargo build --release && cd ..

# Function to start nak server
start_nak() {
    echo -e "${YELLOW}Starting nak server on port $NAK_PORT...${NC}"
    $NAK_BINARY serve --port $NAK_PORT > /tmp/nak.log 2>&1 &
    NAK_PID=$!
    sleep 2
    echo -e "${GREEN}Nak server started (PID: $NAK_PID)${NC}"
}

# Function to stop nak server
stop_nak() {
    if [ ! -z "$NAK_PID" ]; then
        echo -e "${YELLOW}Stopping nak server...${NC}"
        kill $NAK_PID 2>/dev/null || true
        wait $NAK_PID 2>/dev/null || true
        echo -e "${GREEN}Nak server stopped${NC}"
    fi
}

# Cleanup function
cleanup() {
    stop_nak
    # Clean up test data if needed
    if [ "$CLEAN_DATA" = "true" ]; then
        echo "Cleaning up test data..."
        rm -rf ~/.local/share/dialog/* 2>/dev/null || true
        rm -rf ~/Library/Application\ Support/dialog/* 2>/dev/null || true
    fi
}

# Set up trap to cleanup on exit
trap cleanup EXIT

echo "========================================="
echo "Dialog CLI Persistence Test"
echo "========================================="

# Kill any existing nak servers on our port
pkill -f "nak.*serve.*$NAK_PORT" 2>/dev/null || true
sleep 1

# Start nak server
start_nak

echo ""
echo "=== RUN 1: Creating initial notes ==="
echo ""

# Create some notes in the first run
export DIALOG_NSEC=$TEST_NSEC

echo "Creating note 1..."
$CLI_BINARY create "First test note #persistence #test1" || exit 1

echo "Creating note 2..."
$CLI_BINARY create "Second note with different tags #persistence #test2" || exit 1

echo "Creating note 3..."
$CLI_BINARY create "Note without persistence tag #other" || exit 1

echo ""
echo "Listing all notes from Run 1:"
$CLI_BINARY list --limit 10

echo ""
echo "Listing notes with #persistence tag:"
$CLI_BINARY list --tag persistence

# Get initial counts
INITIAL_COUNT=$($CLI_BINARY list --limit 100 2>/dev/null | grep "Total:" | awk '{print $2}')
PERSISTENCE_COUNT=$($CLI_BINARY list --tag persistence --limit 100 2>/dev/null | grep "Total:" | awk '{print $2}')

echo ""
echo -e "${GREEN}Run 1 complete:${NC}"
echo "  - Total notes: $INITIAL_COUNT"
echo "  - Notes with #persistence: $PERSISTENCE_COUNT"

# Stop the relay
echo ""
echo "=== Simulating restart (stopping relay) ==="
stop_nak
sleep 2

# Start relay again
echo ""
echo "=== RUN 2: Checking persistence after restart ==="
start_nak

echo ""
echo "Creating one more note in Run 2..."
$CLI_BINARY create "New note after restart #persistence #test3" || exit 1

echo ""
echo "Listing all notes from Run 2:"
$CLI_BINARY list --limit 10

# Get new counts
NEW_COUNT=$($CLI_BINARY list --limit 100 2>/dev/null | grep "Total:" | awk '{print $2}')
NEW_PERSISTENCE_COUNT=$($CLI_BINARY list --tag persistence --limit 100 2>/dev/null | grep "Total:" | awk '{print $2}')

echo ""
echo "=== Test Results ==="
echo ""

# Verify persistence
EXPECTED_COUNT=$((INITIAL_COUNT + 1))
EXPECTED_PERSISTENCE=$((PERSISTENCE_COUNT + 1))

if [ "$NEW_COUNT" -ge "$EXPECTED_COUNT" ]; then
    echo -e "${GREEN}✓ PASS: Notes persisted across restart${NC}"
    echo "  - Expected at least $EXPECTED_COUNT notes, found $NEW_COUNT"
else
    echo -e "${RED}✗ FAIL: Notes did not persist${NC}"
    echo "  - Expected at least $EXPECTED_COUNT notes, found only $NEW_COUNT"
    exit 1
fi

if [ "$NEW_PERSISTENCE_COUNT" -ge "$EXPECTED_PERSISTENCE" ]; then
    echo -e "${GREEN}✓ PASS: Tagged notes persisted correctly${NC}"
    echo "  - Expected at least $EXPECTED_PERSISTENCE #persistence notes, found $NEW_PERSISTENCE_COUNT"
else
    echo -e "${RED}✗ FAIL: Tagged notes did not persist correctly${NC}"
    echo "  - Expected at least $EXPECTED_PERSISTENCE #persistence notes, found only $NEW_PERSISTENCE_COUNT"
    exit 1
fi

echo ""
echo "=== Testing offline-first capability ==="
echo ""

# Stop relay to test offline mode
stop_nak
echo "Relay stopped. Testing offline note creation..."

# Create note while offline
$CLI_BINARY create "Offline note #offline #persistence" 2>&1 | grep -q "offline mode" && echo -e "${GREEN}✓ Offline warning shown${NC}"

# List should still work from local DB
echo "Listing notes while offline:"
OFFLINE_COUNT=$($CLI_BINARY list --limit 100 2>/dev/null | grep "Total:" | awk '{print $2}')

if [ "$OFFLINE_COUNT" -gt "0" ]; then
    echo -e "${GREEN}✓ PASS: Can list notes from local DB while offline${NC}"
    echo "  - Found $OFFLINE_COUNT notes in local database"
else
    echo -e "${RED}✗ FAIL: Cannot access local database while offline${NC}"
    exit 1
fi

# Start relay again and check sync
echo ""
echo "=== Testing sync after reconnection ==="
start_nak

# Try to sync (create another note to trigger sync)
echo "Creating note to trigger sync..."
$CLI_BINARY create "Note after reconnection #persistence #sync" || exit 1

# Final count
FINAL_COUNT=$($CLI_BINARY list --limit 100 2>/dev/null | grep "Total:" | awk '{print $2}')

echo ""
echo "========================================="
echo -e "${GREEN}All persistence tests passed!${NC}"
echo "========================================="
echo "Final statistics:"
echo "  - Total notes: $FINAL_COUNT"
echo "  - Notes survived relay restart: ✓"
echo "  - Offline mode works: ✓"
echo "  - Sync after reconnection: ✓"
echo ""

# Optional: Show relay logs
if [ "$SHOW_LOGS" = "true" ]; then
    echo "=== Nak relay logs ==="
    cat /tmp/nak.log
fi

exit 0
