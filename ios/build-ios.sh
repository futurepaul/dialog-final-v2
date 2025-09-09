#!/bin/bash
set -e

echo "🔨 Building Dialog iOS App..."

# Change to ios directory if not already there
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Load environment variables if .env exists
if [ -f ".env" ]; then
    echo "📋 Loading environment variables from .env..."
    export $(cat .env | grep -v '^#' | xargs)
else
    echo "⚠️  No .env file found. Copy .env.example to .env and add your DEVELOPMENT_TEAM"
fi

# Clean previous builds
if [ -f "./clean-ios.sh" ]; then
    ./clean-ios.sh
fi

# Check if xcodegen is installed
if ! command -v xcodegen &> /dev/null; then
    echo "❌ XcodeGen is not installed. Installing via Homebrew..."
    brew install xcodegen
fi

# Generate Xcode project
echo "📝 Generating Xcode project..."
xcodegen generate --spec project.yml --use-cache

# Build for simulator
echo "📱 Building for iOS Simulator..."
xcodebuild -project DialogApp.xcodeproj \
  -scheme DialogApp \
  -destination 'platform=iOS Simulator,name=iPhone 16 Pro' \
  build

echo "✅ Build complete!"