#!/bin/bash
echo "🧹 Cleaning iOS build artifacts..."
rm -rf DialogApp.xcodeproj
rm -rf ~/Library/Developer/Xcode/DerivedData/DialogApp-*
rm -rf .xcodegen_cache
echo "✨ Clean complete!"