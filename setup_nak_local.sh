#!/bin/bash

# Setup script for nak with GiftWrap support
# This clones nak, applies our patch, and builds it

set -e

echo "Setting up nak with GiftWrap support..."

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

echo "Applying GiftWrap patch..."
# Copy patch file from script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
if [ -f "$SCRIPT_DIR/nak-giftwrap.patch" ]; then
    git apply "$SCRIPT_DIR/nak-giftwrap.patch"
    echo "Patch applied successfully!"
else
    echo "Warning: nak-giftwrap.patch not found. Building without GiftWrap support."
fi

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