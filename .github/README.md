# GitHub Actions Workflows

This repository uses GitHub Actions for continuous integration and automated releases.

## Workflows

### CI (`.github/workflows/ci.yml`)
Runs on every push and pull request to ensure code quality:
- **Tests** on Linux, macOS, and Windows
- **Formatting** checks with `cargo fmt`
- **Linting** with `cargo clippy`
- **Builds** all components on all platforms

### Release (`.github/workflows/release.yml`)
Triggered when creating version tags (e.g., `v1.0.0`):
- Builds optimized binaries for all platforms
- Creates GitHub release with artifacts
- Publishes packages to registries

## Supported Platforms

### Quickemu Manager
- **GTK4 App**: Linux and macOS (x86_64, aarch64)
- **Dioxus App**: Linux, macOS, and Windows
- **Web App**: Browser-based version

### SPICE Client
- **GTK4 App**: Linux and macOS standalone SPICE viewer
- **Rust Library**: Published to crates.io
- **WASM Package**: Published to npm

## Required Secrets

To enable package publishing, configure these repository secrets:
- `CARGO_REGISTRY_TOKEN`: For publishing to crates.io
- `NPM_TOKEN`: For publishing to npm

## Creating a Release

1. Update version numbers in all `Cargo.toml` files
2. Create and push a version tag:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```
3. The release workflow will automatically:
   - Build binaries for all platforms
   - Create a draft GitHub release
   - Publish packages to crates.io and npm

## Local Development

Use the provided build script for local builds:
```bash
./build-all.sh
```

This will build all components for your current platform.