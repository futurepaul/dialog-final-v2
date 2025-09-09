#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔄 Dialog App Full Rebuild Script"
echo "=================================="
echo ""

# Check if we're in the right directory
if [ ! -f "dialog_uniffi/Cargo.toml" ]; then
    echo -e "${RED}❌ Error: Please run this script from the project root directory${NC}"
    exit 1
fi

# Parse command line arguments
CLEAN_BUILD=false
OPEN_XCODE=false
RUN_APP=false

for arg in "$@"; do
    case $arg in
        --clean)
            CLEAN_BUILD=true
            shift
            ;;
        --open)
            OPEN_XCODE=true
            shift
            ;;
        --run)
            RUN_APP=true
            shift
            ;;
        --help)
            echo "Usage: ./rebuild.sh [options]"
            echo ""
            echo "Options:"
            echo "  --clean    Perform a clean build (removes all artifacts first)"
            echo "  --open     Open Xcode after building"
            echo "  --run      Build and run the app in simulator"
            echo "  --help     Show this help message"
            echo ""
            echo "Examples:"
            echo "  ./rebuild.sh              # Standard rebuild"
            echo "  ./rebuild.sh --clean      # Clean rebuild"
            echo "  ./rebuild.sh --run        # Rebuild and run in simulator"
            echo "  ./rebuild.sh --clean --open  # Clean rebuild and open Xcode"
            exit 0
            ;;
        *)
            echo -e "${YELLOW}⚠️  Unknown option: $arg${NC}"
            ;;
    esac
done

# Clean if requested
if [ "$CLEAN_BUILD" = true ]; then
    echo -e "${YELLOW}🧹 Performing clean build...${NC}"
    
    # Clean Rust artifacts
    echo "  Cleaning Rust build artifacts..."
    cd dialog_uniffi
    cargo clean
    cd ..
    
    # Clean Swift Package artifacts
    echo "  Cleaning Swift Package artifacts..."
    rm -rf DialogPackage/XCFrameworks/*
    rm -rf DialogPackage/Sources/Dialog/*.swift
    rm -rf DialogPackage/Sources/Dialog/*.h
    rm -rf DialogPackage/Sources/Dialog/*.modulemap
    
    # Clean Xcode derived data
    echo "  Cleaning Xcode derived data..."
    rm -rf ~/Library/Developer/Xcode/DerivedData/DialogApp-*
    
    echo -e "${GREEN}✅ Clean complete${NC}"
    echo ""
fi

# Step 1: Build Rust libraries
echo -e "${YELLOW}🦀 Building Rust libraries...${NC}"
cd dialog_uniffi

echo "  📱 Building for iOS device (arm64)..."
cargo build --release --target aarch64-apple-ios

echo "  📱 Building for iOS simulator (arm64)..."
cargo build --release --target aarch64-apple-ios-sim

echo "  💻 Building for macOS (arm64)..."
cargo build --release --target aarch64-apple-darwin

cd ..
echo -e "${GREEN}✅ Rust build complete${NC}"
echo ""

# Step 2: Generate Swift Package
echo -e "${YELLOW}📦 Generating Swift Package...${NC}"
./build-uniffi-package.sh > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Swift Package generated${NC}"
else
    echo -e "${RED}❌ Failed to generate Swift Package${NC}"
    exit 1
fi
echo ""

# Step 3: Generate Xcode project
echo -e "${YELLOW}🔨 Generating Xcode project...${NC}"
cd ios
xcodegen generate --spec project-package.yml > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Xcode project generated${NC}"
else
    echo -e "${RED}❌ Failed to generate Xcode project${NC}"
    exit 1
fi
cd ..
echo ""

# Step 4: Build iOS app
if [ "$RUN_APP" = true ] || [ "$OPEN_XCODE" = false ]; then
    echo -e "${YELLOW}🏗️  Building iOS app...${NC}"
    cd ios
    
    # Try to build for simulator
    xcodebuild -project DialogApp.xcodeproj \
               -scheme DialogApp \
               -destination "platform=iOS Simulator,name=iPhone 16" \
               -configuration Debug \
               build > /tmp/xcode_build.log 2>&1
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ iOS app built successfully${NC}"
    else
        echo -e "${RED}❌ Build failed. Check /tmp/xcode_build.log for details${NC}"
        echo ""
        echo "Last 20 lines of build log:"
        tail -20 /tmp/xcode_build.log
        exit 1
    fi
    cd ..
    echo ""
fi

# Step 5: Run app if requested
if [ "$RUN_APP" = true ]; then
    echo -e "${YELLOW}📱 Running app in simulator...${NC}"
    cd ios
    
    # First, boot the simulator if needed
    xcrun simctl boot "iPhone 16" 2>/dev/null || true
    
    # Open Simulator app
    open -a Simulator
    
    # Install and run the app
    xcodebuild -project DialogApp.xcodeproj \
               -scheme DialogApp \
               -destination "platform=iOS Simulator,name=iPhone 16" \
               -configuration Debug \
               run > /tmp/xcode_run.log 2>&1 &
    
    echo -e "${GREEN}✅ App launching in simulator...${NC}"
    cd ..
fi

# Step 6: Open Xcode if requested
if [ "$OPEN_XCODE" = true ]; then
    echo -e "${YELLOW}📂 Opening Xcode...${NC}"
    open ios/DialogApp.xcodeproj
    echo -e "${GREEN}✅ Xcode opened${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Rebuild complete!${NC}"
echo ""
echo "Next steps:"
if [ "$OPEN_XCODE" = false ] && [ "$RUN_APP" = false ]; then
    echo "  • Open ios/DialogApp.xcodeproj in Xcode"
    echo "  • Select iPhone simulator and press Cmd+R to run"
    echo "  • Or run: ./rebuild.sh --run"
fi
echo ""
echo "Tips:"
echo "  • If you see 'Cannot find type RustBuffer', run: ./rebuild.sh --clean"
echo "  • To see Rust panics, run: export RUST_BACKTRACE=1"
echo "  • Check docs/BUILD_GUIDE.md for detailed documentation"