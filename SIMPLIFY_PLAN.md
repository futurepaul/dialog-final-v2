# Simplification Plan for ios-mock-frontend-le Branch

## Overview
After implementing the UniFFI integration and getting everything working, we have accumulated redundant files, scripts, and documentation. This plan identifies what to remove and consolidate.

## Files to Remove

### 1. Redundant Build Scripts
**Remove:**
- `build-uniffi-ios.sh` - Superseded by `build-uniffi-package.sh`
- `ios.sh` - Old runner script, replaced by `rebuild.sh`
- `ios/build-ios.sh` - Old iOS-only build, replaced by `rebuild.sh`
- `ios/clean-ios.sh` - Cleaning is now part of `rebuild.sh --clean`

**Keep:**
- `rebuild.sh` - Main entry point for all builds
- `build-uniffi-package.sh` - Called by rebuild.sh, handles UniFFI specifics

### 2. Outdated Planning Documents
**Remove:**
- `PLAN_BY_CLAUDE.md` - Initial plan, now implemented
- `PLAN_BY_GPT5.md` - Alternative plan, not used
- `PROMPT.md` - Original prompt, no longer relevant
- `REAL_IMPLEMENTATION_PLAN.md` - Superseded by actual implementation
- `UNIFFI_PLAN.md` - Completed, superseded by BUILD_GUIDE.md
- `FRONTEND_TO_BACKEND_PLAN.md` - Partially implemented, outdated

**Keep:**
- `ASYNC_PROBLEMS.md` - Documents the solution we implemented (historical value)
- `BUILD_GUIDE.md` - Current build documentation
- `CI_SETUP.md` - CI/CD documentation

### 3. XcodeGen Configurations
**Remove:**
- `ios/project.yml` - Old config without Swift Package

**Keep:**
- `ios/project-package.yml` - Current config using Swift Package

### 4. Unnecessary iOS Files
**Remove:**
- `ios/DialogApp/Sources/Models/MockData.swift` - Already deleted but might be referenced
- Any `.xcworkspace` files if committed
- Any `.xcuserdata` directories if committed

## Files to Consolidate

### 1. Documentation
**Create `docs/` directory:**
```
docs/
├── BUILD_GUIDE.md         (move from root)
├── CI_SETUP.md            (move from root)
├── ARCHITECTURE.md        (new - extract from ASYNC_PROBLEMS.md)
└── archive/               
    └── ASYNC_PROBLEMS.md  (historical reference)
```

### 2. Scripts
**Keep in root (these are entry points):**
- `rebuild.sh` - Main build script
- `build-uniffi-package.sh` - UniFFI-specific build (called by rebuild.sh)

## New .gitignore Entries

Add to ensure we don't commit build artifacts:
```gitignore
# Documentation archives
docs/archive/

# Old scripts (remove after cleanup)
*.sh.old

# iOS build artifacts
ios/DialogApp.xcodeproj/
ios/.xcodegen_cache/
```

## Simplification Benefits

### Before: 9 scripts, 9 planning docs, 2 XcodeGen configs
### After: 2 scripts, 3 docs (organized), 1 XcodeGen config

**Reduction:**
- **Scripts:** 78% fewer (9 → 2)
- **Documentation:** 67% fewer (9 → 3)
- **Configs:** 50% fewer (2 → 1)
- **Cognitive Load:** Significantly reduced

## Implementation Steps

1. **Backup current state:**
   ```bash
   git stash  # If any uncommitted changes
   git checkout -b simplify/ios-mock-frontend-le
   ```

2. **Remove redundant scripts:**
   ```bash
   rm build-uniffi-ios.sh
   rm ios.sh
   rm ios/build-ios.sh
   rm ios/clean-ios.sh
   ```

3. **Remove outdated docs:**
   ```bash
   rm PLAN_BY_CLAUDE.md
   rm PLAN_BY_GPT5.md
   rm PROMPT.md
   rm REAL_IMPLEMENTATION_PLAN.md
   rm UNIFFI_PLAN.md
   rm FRONTEND_TO_BACKEND_PLAN.md
   ```

4. **Remove old XcodeGen config:**
   ```bash
   rm ios/project.yml
   ```

5. **Organize documentation:**
   ```bash
   mkdir -p docs/archive
   mv BUILD_GUIDE.md docs/
   mv CI_SETUP.md docs/
   mv ASYNC_PROBLEMS.md docs/archive/
   ```

6. **Create architecture doc:**
   Extract the key architecture decisions from ASYNC_PROBLEMS.md into a clean `docs/ARCHITECTURE.md`

7. **Update references:**
   - Update README.md to point to docs/BUILD_GUIDE.md
   - Update rebuild.sh help text to reference new doc location
   - Update CI workflow if it references any moved files

8. **Rename for clarity:**
   ```bash
   mv ios/project-package.yml ios/project.yml
   ```

9. **Test everything still works:**
   ```bash
   ./rebuild.sh --clean
   ```

10. **Commit simplification:**
    ```bash
    git add -A
    git commit -m "refactor: Simplify project structure and remove redundant files
    
    - Consolidated 9 scripts down to 2
    - Organized docs into docs/ directory  
    - Removed 6 outdated planning documents
    - Removed redundant XcodeGen config
    - Clear separation: rebuild.sh for users, build-uniffi-package.sh for internals"
    ```

## Risk Assessment

**Low Risk:** 
- All removed files are either redundant or outdated
- Core functionality preserved in `rebuild.sh`
- Documentation improved, not lost

**Mitigation:**
- All changes on separate branch first
- Test build after each major removal
- Can always reference git history if needed

## Recommendation

**PROCEED WITH SIMPLIFICATION**

The benefits far outweigh the minimal risks. This will make the project much easier to understand and maintain.