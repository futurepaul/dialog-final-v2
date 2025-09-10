# Build Guide for Dialog UniFFI iOS Integration

This guide documents the build process for the Dialog app with a Rust backend via UniFFI.

## Prerequisites

### Required Tools
- Xcode 15+ with iOS SDK
- Rust with iOS targets installed
- XcodeGen (`brew install xcodegen`)
- UniFFI (included in dialog_uniffi crate)

### Install iOS Rust Targets
```bash
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim  
rustup target add aarch64-apple-darwin    # For macOS support
```

## Project Structure

```
repo/
├── dialog_uniffi/              # Rust crate with UniFFI bindings
│   ├── Cargo.toml
│   ├── build.rs               # UniFFI scaffolding generation
│   ├── src/
│   │   ├── dialog.udl         # UniFFI interface definition
│   │   ├── lib.rs            # Rust implementation
│   │   ├── models.rs         # Shared data models
│   │   └── mock_data.rs      # Mock data generator
│   └── target/               # Build artifacts (gitignored)
├── ios/DialogPackage/         # Swift Package wrapping UniFFI
│   ├── Package.swift
│   ├── Sources/Dialog/        # Generated Swift code (UniFFI)
│   └── XCFrameworks/          # Binary framework (XCFramework)
├── ios/                      # iOS app
│   ├── project.yml           # XcodeGen config using Swift Package
│   └── DialogApp/            # App source code
└── build-uniffi-package.sh   # Packaging script (bindings + XCFramework)

```

## Build Process

### Quick Start

- Build package + app: `just ios` (runs packaging and builds the app)
- Faster: `just ios-fast` (reuses Rust artifacts)
- Only regenerate bindings/XCFramework: `just package` or `just package-fast`

## Clean Build Process

If you encounter issues, run:

```bash
just clean-ios && just ios
```

This automatically:
- Cleans Rust build artifacts
- Removes Swift Package generated files
- Clears Xcode derived data
- Rebuilds everything from scratch

## Incremental Builds

### After Changes

- Rust: `just package` or `just ios`
- Swift: Xcode build (Cmd+R), or `just ios-fast`
- UDL: `just package` (bindings regenerate automatically)

## Important Details

### Module Naming

The C module must be properly exposed for Swift to import it:

1. **module.modulemap**: Must be named exactly `module.modulemap` (not `dialogFFI.modulemap`)
2. **Module location**: Must be in the Headers directory of each XCFramework slice
3. **Module name**: The module declares itself as `dialogFFI` which matches the Swift import

### Architecture Configuration

The project is configured for ARM64 only:
- **Excluded**: x86_64 (Intel simulators not supported)
- **Included**: arm64 (Apple Silicon Macs, all modern iOS devices)

This is set in `ios/project.yml`:
```yaml
settings:
  EXCLUDED_ARCHS[sdk=iphonesimulator*]: x86_64
  VALID_ARCHS: arm64
```

### Swift Concurrency

The implementation uses a **synchronous, fire-and-forget pattern** to avoid Swift 6 concurrency issues:

- Rust methods are synchronous but spawn work internally on Tokio runtime
- No `async`/`await` in Swift code
- Callbacks handle thread transitions with `Task { @MainActor in ... }`
- Avoids all Sendable-related compilation errors

### Common Issues and Solutions

#### "Cannot find type 'RustBuffer' in scope"

**Cause**: The dialogFFI C module isn't being imported properly.

**Solution**: 
1. Ensure `module.modulemap` exists in XCFramework Headers
2. Check that Swift file has `import dialogFFI`
3. Rebuild with `./build-uniffi-package.sh`

#### "Cannot block the current thread from within a runtime"

**Cause**: Using `blocking_read()` within Tokio async context.

**Solution**: Already fixed - we use `try_read()` for synchronous access.

#### "Could not find module 'Dialog' for target 'x86_64-apple-ios-simulator'"

**Cause**: Trying to build for Intel simulator architecture.

**Solution**: Already fixed - x86_64 is excluded in project settings.

#### Build succeeds but no data appears

**Cause**: The listener might not be properly connected.

**Solution**: Check that `client.start(listener)` is called in the ViewModel.

## Testing Changes

After any build:

1. Run the app in simulator
2. Verify the inbox loads with mock data from Rust
3. Test creating notes and switching between tags
4. Check Xcode console for any runtime errors

## Continuous Integration

For CI/CD, the build can be automated:

```bash
#!/bin/bash
set -e

# Install dependencies
rustup target add aarch64-apple-ios aarch64-apple-ios-sim

# Build
./build-uniffi-package.sh

# Generate Xcode project
cd ios
xcodegen generate --spec project-package.yml

# Build for testing
xcodebuild -project DialogApp.xcodeproj \
           -scheme DialogApp \
           -destination "generic/platform=iOS" \
           build-for-testing
```

## Debugging Tips

### Enable Rust Backtraces

If you see a panic without details:
```bash
export RUST_BACKTRACE=1
# Then run the app
```

### Check Generated Files

Verify the generated files are correct:
```bash
# Check Swift bindings
ls -la ios/DialogPackage/Sources/Dialog/

# Check XCFramework structure  
ls -la ios/DialogPackage/XCFrameworks/

# Verify module.modulemap exists
cat ios/DialogPackage/XCFrameworks/*/ios-arm64/Headers/module.modulemap
```

### Xcode Build Logs

For detailed error messages:
1. In Xcode: View → Navigators → Report Navigator
2. Click on the failed build
3. Click on individual steps to see full output

## Notes
- The app currently connects to `wss://relay.damus.io` by default (see InboxViewModel).
- The packaging script auto-bumps the XCFramework version and validates UniFFI contracts.
