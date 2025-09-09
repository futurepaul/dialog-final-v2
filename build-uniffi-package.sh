#!/bin/bash
set -e

echo "🦀 Building UniFFI for Swift Package..."

cd dialog_uniffi

# Build for iOS architectures
echo "📱 Building for iOS device (arm64)..."
cargo build --release --target aarch64-apple-ios

echo "📱 Building for iOS simulator (arm64)..."
cargo build --release --target aarch64-apple-ios-sim

echo "💻 Building for macOS (arm64)..."
cargo build --release --target aarch64-apple-darwin

# Clean previous generated files
rm -rf ../DialogPackage/Sources/Dialog/*.swift
rm -rf ../DialogPackage/XCFrameworks/*.xcframework

# Generate Swift bindings into the package
echo "🔨 Generating Swift bindings..."
cargo run --features bindgen-support --bin uniffi-bindgen generate \
    --library target/aarch64-apple-ios/release/libdialog_uniffi.a \
    --language swift \
    --out-dir ../DialogPackage/Sources/Dialog \
    src/dialog.udl

# Post-process the generated Swift file to ensure unconditional import
echo "🔧 Post-processing Swift bindings..."
# Remove the conditional import and replace with direct import
sed -i '' '/#if canImport(dialogFFI)/,/#endif/c\
import dialogFFI' ../DialogPackage/Sources/Dialog/dialog.swift


# Create headers directory for XCFramework
mkdir -p ../DialogPackage/Headers

# Copy header to headers directory
cp ../DialogPackage/Sources/Dialog/dialogFFI.h ../DialogPackage/Headers/
# Rename modulemap to module.modulemap (required by Xcode)
cp ../DialogPackage/Sources/Dialog/dialogFFI.modulemap ../DialogPackage/Headers/module.modulemap

# Create XCFramework with all architectures
echo "📦 Creating XCFramework..."
xcodebuild -create-xcframework \
    -library target/aarch64-apple-ios/release/libdialog_uniffi.a \
    -headers ../DialogPackage/Headers \
    -library target/aarch64-apple-ios-sim/release/libdialog_uniffi.a \
    -headers ../DialogPackage/Headers \
    -library target/aarch64-apple-darwin/release/libdialog_uniffi.a \
    -headers ../DialogPackage/Headers \
    -output ../DialogPackage/XCFrameworks/dialogFFI.xcframework

# Clean up temporary headers directory and header/modulemap from Sources
rm -rf ../DialogPackage/Headers
rm -f ../DialogPackage/Sources/Dialog/dialogFFI.h
rm -f ../DialogPackage/Sources/Dialog/dialogFFI.modulemap

echo "✅ Swift Package build complete!"
echo "📁 Generated files:"
echo "   - Swift Package: DialogPackage/"
echo "   - Swift bindings: DialogPackage/Sources/Dialog/"
echo "   - XCFramework: DialogPackage/XCFrameworks/dialogFFI.xcframework"
echo ""
echo "📦 To use in your iOS app:"
echo "   1. In Xcode: File > Add Package Dependencies"
echo "   2. Add Local Package: Select the DialogPackage folder"
echo "   3. Import Dialog in your Swift files"