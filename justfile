# Dialog build commands

# Default: show available commands
default:
    @just --list

# Build and run iOS app
ios: 
    bash build-uniffi-package.sh
    cd ios && (command -v xcodegen >/dev/null 2>&1 && xcodegen generate || echo "xcodegen not found, using existing .xcodeproj") \
        && xcodebuild -scheme DialogApp \
        -destination 'platform=iOS Simulator,name=iPhone 16,OS=18.4' \
        build

# Generate Swift package (bindings + XCFrameworks)
package:
    bash build-uniffi-package.sh

# Generate Swift package without rebuilding Rust
package-fast:
    SKIP_RUST=1 bash build-uniffi-package.sh

# Build rust
build:
    cargo build --release

# Run CLI
cli *ARGS:
    cargo run -p dialog_cli -- {{ARGS}}

# Run tests
test:
    cargo test

# Clean everything
clean:
    cargo clean
    # Remove generated bindings and frameworks, keep Package.swift
    rm -rf ios/DialogPackage/XCFrameworks/*
    rm -rf ios/DialogPackage/Sources/Dialog/*.swift
    rm -rf ios/DialogPackage/Sources/Dialog/*.h
    rm -rf ios/DialogPackage/Sources/Dialog/*.modulemap
    # Skip Xcode clean here to avoid SPM resolving the (now-missing) binary target.
    # Run `just package` first, then use Xcode's clean if needed.
    # Remove any legacy root-level DialogPackage to avoid Xcode confusion
    rm -rf DialogPackage

# Clean only iOS packaging artifacts (keeps Rust target/ caches)
clean-ios:
    rm -rf ios/DialogPackage/XCFrameworks/*
    rm -rf ios/DialogPackage/Sources/Dialog/*.swift
    rm -rf ios/DialogPackage/Sources/Dialog/*.h
    rm -rf ios/DialogPackage/Sources/Dialog/*.modulemap

# Build iOS app quickly (reuse existing Rust artifacts)
ios-fast:
    SKIP_RUST=1 bash build-uniffi-package.sh
    cd ios && (command -v xcodegen >/dev/null 2>&1 && xcodegen generate || echo "xcodegen not found, using existing .xcodeproj") \
        && xcodebuild -scheme DialogApp \
        -destination 'platform=iOS Simulator,name=iPhone 16,OS=18.4' \
        build

# Bump XCFramework version to force SPM cache refresh, then rebuild package
bump-package:
    bash -lc 'vfile=ios/DialogPackage/.xcframework-version; v=$([ -f "$vfile" ] && cat "$vfile" || echo 1); nv=$((v+1)); echo $nv > "$vfile"; echo "Bumped XCFramework version to $nv"'
    bash build-uniffi-package.sh

# Format and check
check:
    cargo fmt
    cargo clippy
