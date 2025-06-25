# Quickemu Manager - Build Commands
default:
    @just --list

# Build all applications
build:
    cargo build --workspace

# Build the GTK4 desktop application
build-gtk:
    cargo build -p quickemu-manager-gtk

# Build the Dioxus desktop application
build-dioxus-desktop:
    cargo build -p quickemu-manager-ui --features desktop

# Build the Dioxus web server
build-dioxus-server:
    cargo build -p quickemu-manager-ui --features server --no-default-features

# Build the core library
build-core:
    cargo build -p quickemu-core

# Build everything in release mode
release:
    cargo build --workspace --release
    cd dioxus-app && dx build --platform web --release

# Run the GTK4 desktop application
run-gtk:
    cd gtk4-app && cargo run

# Run the Dioxus desktop application
run-dioxus:
    cd dioxus-app && cargo run --features desktop

# Run the Dioxus web server
run-server:
    cd dioxus-app && cargo run --features server --no-default-features

# Run the Dioxus web application (development)
web:
    cd dioxus-app && dx serve --platform web

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
    cd dioxus-app && dx clean

# Quick CI check
ci: fmt lint test
    @echo "All checks passed! ✅"

# Build in distrobox (for GTK4 dependencies)
distrobox-build:
    distrobox-enter --name ubuntu-24.04 -- cargo build --workspace

# Run GTK4 app in distrobox
distrobox-run:
    distrobox-enter --name ubuntu-24.04 -- cargo run -p quickemu-manager-gtk