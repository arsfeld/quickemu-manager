# Quickemu Manager - Build Commands
default:
    @just --list

# Build all applications
build:
    cargo build --workspace

# Build the GTK4 desktop application
build-gtk:
    cargo build -p quickemu-manager-gtk

# Build the Slint desktop application
build-slint:
    cargo build -p quickemu-manager

# Build the core library
build-core:
    cargo build -p quickemu-core

# Build everything in release mode
build-release:
    cargo build --workspace --release

# Run the GTK4 desktop application
run-gtk:
    cd gtk4-app && cargo run

# Run the Slint desktop application
run-slint:
    cd slint-app && cargo run

# Run the Slint desktop application with verbose logging
run-slint-verbose:
    cd slint-app && RUST_LOG=info cargo run

# Run all tests
test:
    cargo test --workspace

# Format code
fmt:
    cargo fmt --all

# Run clippy lints
lint:
    cargo clippy --workspace -- -D warnings

# Clean build artifacts
clean:
    cargo clean

# Quick CI check
ci: fmt lint test
    @echo "All checks passed! ✅"

# Build in distrobox (for GTK4 dependencies)
distrobox-build:
    distrobox-enter --name ubuntu-24.04 -- cargo build --workspace

# Run GTK4 app in distrobox
distrobox-run:
    distrobox-enter --name ubuntu-24.04 -- cargo run -p quickemu-manager-gtk

# Create a new release with LLM-generated release notes and version suggestion
release version="" llm="claude":
    @scripts/release.py {{version}} --llm {{llm}}

# List recent releases
list-releases:
    @echo "Recent releases:"
    @gh release list --limit 10