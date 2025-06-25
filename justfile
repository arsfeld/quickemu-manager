# Quickemu Manager - Build Commands
default:
    @just --list

# Build the GTK4 desktop application
build:
    cargo build

# Build everything in release mode
release:
    cargo build --release
    cd dioxus-app && dx build --platform web --release

# Run the GTK4 desktop application
run:
    cargo run

# Run the Dioxus web application
web:
    cd dioxus-app && dx serve --platform web

# Run all tests
test:
    cargo test

# Format code
fmt:
    cargo fmt
    cd dioxus-app && cargo fmt

# Run clippy lints
lint:
    cargo clippy -- -D warnings
    cd dioxus-app && cargo clippy -- -D warnings

# Clean build artifacts
clean:
    cargo clean
    cd dioxus-app && cargo clean
    cd dioxus-app && dx clean

# Quick CI check
ci: fmt lint test
    @echo "All checks passed! ✅"