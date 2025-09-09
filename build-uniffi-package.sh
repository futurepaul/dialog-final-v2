#!/bin/bash
set -e

echo "🦀 Building UniFFI for Swift Package (workspace root -> ios/DialogPackage)..."

# Set deployment targets for iOS 18.4+ support (required by nostrdb)
export IPHONEOS_DEPLOYMENT_TARGET=18.4
export MACOSX_DEPLOYMENT_TARGET=14.0

if [ -z "$SKIP_RUST" ]; then
  # Build for iOS architectures
  echo "📱 Building for iOS device (arm64)..."
cargo build -p dialog_uniffi --release --target aarch64-apple-ios

  echo "📱 Building for iOS simulator (arm64)..."
cargo build -p dialog_uniffi --release --target aarch64-apple-ios-sim

  echo "💻 Building for macOS (arm64)..."
cargo build -p dialog_uniffi --release --target aarch64-apple-darwin
else
  echo "⏭️  SKIP_RUST=1 set; skipping Rust compilation and using existing artifacts"
fi

# Ensure output directories exist
mkdir -p ios/DialogPackage/Sources/Dialog
mkdir -p ios/DialogPackage/XCFrameworks

# Clean previous generated files
rm -rf ios/DialogPackage/Sources/Dialog/*.swift
rm -rf ios/DialogPackage/XCFrameworks/*.xcframework

# Generate Swift bindings into the package
echo "🔨 Generating Swift bindings..."
cargo run -p dialog_uniffi --features bindgen-support --bin uniffi-bindgen generate \
    --library target/aarch64-apple-ios/release/libdialog_uniffi.a \
    --language swift \
    --out-dir ios/DialogPackage/Sources/Dialog \
    dialog_uniffi/src/dialog.udl

# Post-process the generated Swift file to ensure unconditional import
echo "🔧 Post-processing Swift bindings..."
# Remove the conditional import and replace with direct import
sed -i '' '/#if canImport(dialogFFI)/,/#endif/c\
import dialogFFI' ios/DialogPackage/Sources/Dialog/dialog.swift


# Validate UniFFI contract/checksums between Swift and Rust scaffolding
echo "🧪 Validating UniFFI contract/checksums..."
# Locate Rust-generated scaffolding file for the iOS target
RUST_SCAFF=$(ls target/aarch64-apple-ios-sim/release/build/dialog_uniffi-*/out/dialog.uniffi.rs 2>/dev/null | head -n1)
if [ -z "$RUST_SCAFF" ]; then
  RUST_SCAFF=$(ls target/aarch64-apple-ios/release/build/dialog_uniffi-*/out/dialog.uniffi.rs 2>/dev/null | head -n1)
fi
if [ -z "$RUST_SCAFF" ]; then
  echo "❌ Could not locate Rust scaffolding (dialog.uniffi.rs)." >&2
  exit 1
fi

# Compare checksum function name sets
SWIFT_CHECKS=$(grep -o 'uniffi_dialog_uniffi_checksum_[A-Za-z0-9_]*' ios/DialogPackage/Sources/Dialog/dialog.swift | sort -u || true)
RUST_CHECKS=$(grep -o 'uniffi_dialog_uniffi_checksum_[A-Za-z0-9_]*' "$RUST_SCAFF" | sort -u || true)
if ! diff -u <(echo "$SWIFT_CHECKS") <(echo "$RUST_CHECKS") >/dev/null; then
  echo "❌ UniFFI checksum function set mismatch between Swift and Rust scaffolding." >&2
  echo "Swift-only:" >&2; comm -23 <(echo "$SWIFT_CHECKS") <(echo "$RUST_CHECKS") >&2 || true
  echo "Rust-only:" >&2; comm -13 <(echo "$SWIFT_CHECKS") <(echo "$RUST_CHECKS") >&2 || true
  exit 1
fi

# Quick sanity: Swift bindings contract version is 29 for uniffi 0.29
if ! grep -q 'let bindings_contract_version = 29' ios/DialogPackage/Sources/Dialog/dialog.swift; then
  echo "❌ Unexpected bindings contract version in Swift (expected 29)." >&2
  exit 1
fi
echo "✅ UniFFI validation passed."


# Create headers directory for XCFramework
mkdir -p ios/DialogPackage/Headers

# Copy header to headers directory
cp ios/DialogPackage/Sources/Dialog/dialogFFI.h ios/DialogPackage/Headers/
# Rename modulemap to module.modulemap (required by Xcode)
cp ios/DialogPackage/Sources/Dialog/dialogFFI.modulemap ios/DialogPackage/Headers/module.modulemap

# Create XCFramework with all architectures
echo "📦 Creating XCFramework..."
# Versioned output to bust Xcode package cache when needed
VER_FILE=ios/DialogPackage/.xcframework-version
if [ ! -f "$VER_FILE" ]; then echo 1 > "$VER_FILE"; fi
VER=$(cat "$VER_FILE")
OUT_NAME="dialogFFI_v${VER}.xcframework"

# Clean prior XCFrameworks
rm -rf ios/DialogPackage/XCFrameworks/*.xcframework

xcodebuild -create-xcframework \
    -library target/aarch64-apple-ios/release/libdialog_uniffi.a \
    -headers ios/DialogPackage/Headers \
    -library target/aarch64-apple-ios-sim/release/libdialog_uniffi.a \
    -headers ios/DialogPackage/Headers \
    -library target/aarch64-apple-darwin/release/libdialog_uniffi.a \
    -headers ios/DialogPackage/Headers \
    -output ios/DialogPackage/XCFrameworks/${OUT_NAME}

# Clean up temporary headers directory and header/modulemap from Sources
rm -rf ios/DialogPackage/Headers
rm -f ios/DialogPackage/Sources/Dialog/dialogFFI.h
rm -f ios/DialogPackage/Sources/Dialog/dialogFFI.modulemap

echo "✅ Swift Package build complete!"
echo "📁 Generated files:"
echo "   - Swift Package: ios/DialogPackage/"
echo "   - Swift bindings: ios/DialogPackage/Sources/Dialog/"
echo "   - XCFramework: ios/DialogPackage/XCFrameworks/${OUT_NAME}"
echo ""
echo "🔁 Updating Package.swift to point at ${OUT_NAME}"
# Update Package.swift binary target path
sed -i '' "s#path: \"XCFrameworks/.*dialogFFI.*\.xcframework\"#path: \"XCFrameworks/${OUT_NAME}\"#" ios/DialogPackage/Package.swift
echo ""
echo "📦 To use in your iOS app:"
echo "   1. In Xcode: File > Add Package Dependencies"
echo "   2. Add Local Package: Select the ios/DialogPackage folder"
echo "   3. Import Dialog in your Swift files"
