# Build Guide for Dialog UniFFI iOS Integration

This guide documents the complete build process for the Dialog app with Rust backend via UniFFI.

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
ios-mock-frontend-le/
├── dialog_uniffi/              # Rust crate with UniFFI bindings
│   ├── Cargo.toml
│   ├── build.rs               # UniFFI scaffolding generation
│   ├── src/
│   │   ├── dialog.udl         # UniFFI interface definition
│   │   ├── lib.rs            # Rust implementation
│   │   ├── models.rs         # Shared data models
│   │   └── mock_data.rs      # Mock data generator
│   └── target/               # Build artifacts (gitignored)
├── DialogPackage/             # Swift Package wrapping UniFFI
│   ├── Package.swift
│   ├── Sources/Dialog/        # Generated Swift code
│   └── XCFrameworks/         # Binary framework (gitignored)
├── ios/                      # iOS app
│   ├── project-package.yml   # XcodeGen config using Swift Package
│   └── DialogApp/            # App source code
└── build-uniffi-package.sh   # Main build script

```

## Build Process

### Quick Start - One Command

After making changes to Rust code, simply run:

```bash
./rebuild.sh
```

This single script handles everything:
1. Builds Rust libraries for all targets
2. Generates Swift bindings via UniFFI  
3. Creates XCFramework with proper module setup
4. Generates Xcode project
5. Builds the iOS app

### Build Options

```bash
# Standard rebuild after Rust changes
./rebuild.sh

# Clean rebuild (removes all artifacts first)
./rebuild.sh --clean

# Rebuild and run in simulator
./rebuild.sh --run

# Rebuild and open Xcode
./rebuild.sh --open

# Clean rebuild and open Xcode
./rebuild.sh --clean --open

# Show all options
./rebuild.sh --help
```

## Clean Build Process

If you encounter issues, just run:

```bash
./rebuild.sh --clean
```

This automatically:
- Cleans Rust build artifacts
- Removes Swift Package generated files
- Clears Xcode derived data
- Rebuilds everything from scratch

## Incremental Builds

### After Rust Changes

```bash
./rebuild.sh
```

### After Swift Changes

Just rebuild in Xcode (Cmd+R) - no need to run the script.

### After UDL Changes

If you modified `dialog.udl`, run:

```bash
./rebuild.sh --clean
```

The UDL changes require a full rebuild to ensure proper code generation.

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

This is set in `ios/project-package.yml`:
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
ls -la DialogPackage/Sources/Dialog/

# Check XCFramework structure  
ls -la DialogPackage/XCFrameworks/dialogFFI.xcframework/ios-arm64/Headers/

# Verify module.modulemap exists
cat DialogPackage/XCFrameworks/dialogFFI.xcframework/ios-arm64/Headers/module.modulemap
```

### Xcode Build Logs

For detailed error messages:
1. In Xcode: View → Navigators → Report Navigator
2. Click on the failed build
3. Click on individual steps to see full output

## Next Steps

To connect to the real backend instead of mock data:

1. Update `dialog_uniffi/src/lib.rs` to connect to actual services
2. Remove mock_data.rs or make it conditional
3. Add network error handling
4. Implement proper authentication if needed

The architecture is ready - just replace the mock implementation!