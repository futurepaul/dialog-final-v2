#!/bin/bash
set -e

echo "🦀 Building UniFFI for iOS..."

cd dialog_uniffi

# Build for iOS architectures
echo "📱 Building for iOS device (arm64)..."
cargo build --release --target aarch64-apple-ios

echo "📱 Building for iOS simulator (arm64)..."
cargo build --release --target aarch64-apple-ios-sim

# Create directories for generated files
mkdir -p ../ios/DialogApp/Generated
mkdir -p ../ios/DialogApp/Frameworks

# Generate Swift bindings using cargo run with feature flag
echo "🔨 Generating Swift bindings..."
cargo run --features bindgen-support --bin uniffi-bindgen generate \
    --library target/aarch64-apple-ios/release/libdialog_uniffi.a \
    --language swift \
    --out-dir ../ios/DialogApp/Generated \
    src/dialog.udl

# Create XCFramework with both device and simulator
echo "📦 Creating XCFramework..."
xcodebuild -create-xcframework \
    -library target/aarch64-apple-ios/release/libdialog_uniffi.a \
    -headers ../ios/DialogApp/Generated/dialogFFI.h \
    -library target/aarch64-apple-ios-sim/release/libdialog_uniffi.a \
    -headers ../ios/DialogApp/Generated/dialogFFI.h \
    -output ../ios/DialogApp/Frameworks/DialogFFI.xcframework

echo "✅ UniFFI iOS build complete!"
echo "📁 Generated files:"
echo "   - Swift bindings: ios/DialogApp/Generated/"
echo "   - XCFramework: ios/DialogApp/Frameworks/DialogFFI.xcframework"