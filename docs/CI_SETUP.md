# CI Setup for iOS Builds

## GitHub Actions Workflow

The project includes a GitHub Actions workflow that automatically builds the iOS app on every push and pull request. This serves as a smoke test to ensure the Rust → UniFFI → Swift → iOS pipeline is working correctly.

## What the CI Does

1. **Builds Rust libraries** for iOS device, simulator, and macOS
2. **Generates Swift bindings** via UniFFI
3. **Creates XCFramework** with proper module setup
4. **Generates Xcode project** using XcodeGen
5. **Builds the iOS app** for simulator
6. **Verifies artifacts** were created correctly

## Workflow Triggers

The workflow runs when:
- Code is pushed to `main` or `ios-mock-frontend-le` branches
- Pull requests are opened against `main`
- Manual trigger via GitHub Actions UI
- Only when relevant files change (Rust code, iOS code, build scripts)

## Requirements

### GitHub Runners

The workflow uses **macOS-14 runners** (Apple Silicon M1) which:
- Match our arm64 architecture targets
- Have Xcode pre-installed
- Run faster than Intel runners
- Cost the same as Intel runners

### No Secrets Required! 

For debug builds (what we're doing), you don't need:
- ❌ Apple Developer account
- ❌ Signing certificates
- ❌ Provisioning profiles
- ❌ App Store Connect API keys

The app builds with automatic signing disabled for simulator only.

## If You Want to Add Distribution Later

If you later want to build for real devices or distribute, you'll need to add these secrets to your GitHub repository:

```yaml
# In Settings → Secrets and variables → Actions

# For development builds on real devices:
DEVELOPMENT_TEAM_ID        # Your Apple Developer Team ID
APPLE_CERTIFICATE_BASE64   # Base64 encoded .p12 certificate
CERTIFICATE_PASSWORD       # Password for the certificate

# For App Store distribution:
APP_STORE_CONNECT_KEY_ID   # API Key ID
APP_STORE_CONNECT_ISSUER_ID # Issuer ID  
APP_STORE_CONNECT_KEY      # Private key content
```

But again, **none of these are needed** for the current smoke test builds!

## Running CI Locally

To test the CI workflow locally, you can use [act](https://github.com/nektos/act):

```bash
# Install act
brew install act

# Run the workflow locally
act -P macos-14=-self-hosted
```

Or run the same steps locally via justfile:

```bash
just clean-ios
just package
cd ios && xcodegen generate && cd ..
just ios-fast
```

## Monitoring CI

### View Workflow Runs

Go to: https://github.com/[your-username]/[repo-name]/actions

### Download Build Logs

If a build fails, the workflow automatically uploads logs as artifacts. You can download them from the workflow run page.

### Badge for README

Add this to your main README to show build status:

```markdown
[![iOS Build](https://github.com/[your-username]/[repo-name]/actions/workflows/ios-build.yml/badge.svg)](https://github.com/[your-username]/[repo-name]/actions/workflows/ios-build.yml)
```

## Troubleshooting CI Failures

### Common Issues

1. **"No such module 'dialogFFI'"**
   - The XCFramework wasn't built correctly
   - Check the build logs for Rust compilation errors

2. **"Could not find module for target x86_64"**
   - This is expected - we only build for arm64
   - The workflow should handle this automatically

3. **"Command XcodeGen not found"**
   - Shouldn't happen on GitHub runners
   - The workflow installs it automatically

4. **Timeout Issues**
   - First runs take longer due to Rust compilation
   - Subsequent runs use cache and are faster
   - Full build typically takes 5-10 minutes

### Debugging Steps

1. Check the "Build iOS App" step output
2. Download build logs artifact if available
3. Try running `./rebuild.sh --clean` locally
4. Compare local vs CI environment versions

## Performance Optimization

The workflow includes caching for:
- Rust dependencies and build artifacts
- Speeds up builds from ~10 minutes to ~3-5 minutes

## Future Enhancements

Potential improvements to add:
- [ ] Run SwiftUI preview tests
- [ ] Build for multiple iOS versions
- [ ] Generate and upload IPA artifact
- [ ] Run UI tests with XCUITest
- [ ] Integration with Fastlane for distribution
- [ ] Slack/Discord notifications on failure

But the current setup provides a solid smoke test that ensures the entire build pipeline works!
