#!/bin/bash

# Setup git hooks for the dialog project

echo "Setting up git hooks..."

# Configure git to use .githooks directory
git config core.hooksPath .githooks

echo "✅ Git hooks configured!"
echo ""
echo "Pre-commit hook will now:"
echo "  - Check Rust formatting with 'cargo fmt --check'"
echo "  - Run clippy (warnings only, non-blocking)"
echo ""
echo "To bypass hooks in emergency: git commit --no-verify"