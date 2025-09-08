#!/bin/bash

# Setup script for nak relay
# This clones nak and builds it

set -e

echo "Setting up nak relay..."

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

echo "Building nak..."
go build -o nak

echo "Installing nak to ~/go/bin/..."
mkdir -p ~/go/bin
cp nak ~/go/bin/

# Clean up
cd /
rm -rf "$TEMP_DIR"

echo "Done! nak is installed at ~/go/bin/nak"
echo ""
echo "To run the local relay server:"
echo "  ~/go/bin/nak serve --port 10548"
echo ""
echo "Or add ~/go/bin to your PATH and run:"
echo "  nak serve --port 10548"
echo ""
echo "Note: nak already supports all event kinds including GiftWrap (1059)"