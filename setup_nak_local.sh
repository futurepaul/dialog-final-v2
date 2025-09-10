#!/bin/bash

# Setup script for nak relay with negentropy support
# This clones nak, applies negentropy patch, and builds it

set -e

echo "Setting up nak relay..."

# Resolve repository root (directory containing this script)
REPO_ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Check if go is installed
if ! command -v go &> /dev/null; then
    echo "Error: Go is not installed. Please install Go first."
    exit 1
fi

# Create temp directory for build
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

echo "Cloning nak repository..."
git clone https://github.com/fiatjaf/nak.git
cd nak

# Apply negentropy patch if it exists
if [ -f "$REPO_ROOT/nak-negentropy.patch" ]; then
    echo "Applying negentropy patch..."
    git apply "$REPO_ROOT/nak-negentropy.patch"
    echo "Patch applied successfully!"
fi

echo "Building nak with negentropy support..."
go build -o nak

# Place the patched binary at the repo root as 'nak-negentropy'
cp nak "$REPO_ROOT/nak-negentropy"

# Clean up
cd /
rm -rf "$TEMP_DIR"

echo "Done! Patched nak placed at: $REPO_ROOT/nak-negentropy"
echo ""
echo "To run the local relay server manually:"
echo "  ./nak-negentropy serve --port 10548"
echo ""
echo "Note: Tests expect './nak-negentropy' in the repo root."
