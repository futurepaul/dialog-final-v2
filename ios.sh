#!/bin/bash
# Convenience script to run iOS commands from root

if [ "$1" = "build" ]; then
    ./ios/build-ios.sh
elif [ "$1" = "clean" ]; then
    ./ios/clean-ios.sh
elif [ "$1" = "open" ]; then
    open ios/DialogApp.xcodeproj
else
    echo "Usage: ./ios.sh [build|clean|open]"
    echo "  build - Build the iOS app"
    echo "  clean - Clean build artifacts"
    echo "  open  - Open in Xcode"
fi