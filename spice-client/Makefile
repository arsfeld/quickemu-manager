.PHONY: help test test-unit test-integration test-all test-wasm bench clean docker-build docker-up docker-down check fmt lint docs

# Default target
help:
	@echo "SPICE Client Development Commands"
	@echo "================================="
	@echo "make test          - Run all tests"
	@echo "make test-unit     - Run unit tests only"
	@echo "make test-integration - Run integration tests"
	@echo "make test-wasm     - Run WASM tests"
	@echo "make bench         - Run benchmarks"
	@echo "make check         - Check code without building"
	@echo "make fmt           - Format code"
	@echo "make lint          - Run clippy linter"
	@echo "make docs          - Generate documentation"
	@echo "make clean         - Clean build artifacts"
	@echo ""
	@echo "Docker Commands:"
	@echo "make docker-build  - Build test Docker image"
	@echo "make docker-up     - Start test Docker container"
	@echo "make docker-down   - Stop test Docker container"

# Run all tests
test: test-unit test-integration

# Run unit tests
test-unit:
	@echo "Running unit tests..."
	@cargo test --lib --all-features

# Run integration tests
test-integration:
	@echo "Running integration tests..."
	@./run-integration-tests.sh

# Run all tests including integration
test-all:
	@./run-integration-tests.sh

# Run WASM tests
test-wasm:
	@echo "Building for WASM..."
	@cargo build --target wasm32-unknown-unknown --all-features
	@echo "Running WASM tests..."
	@wasm-pack test --headless --chrome

# Run benchmarks
bench:
	@echo "Running benchmarks..."
	@cargo bench

# Check code without building
check:
	@cargo check --all-features
	@cargo check --target wasm32-unknown-unknown --all-features

# Format code
fmt:
	@cargo fmt

# Run linter
lint:
	@cargo clippy --all-features -- -D warnings

# Generate documentation
docs:
	@cargo doc --all-features --no-deps --open

# Clean build artifacts
clean:
	@cargo clean
	@rm -rf target/
	@rm -rf pkg/
	@rm -rf wasm-pack.log

# Docker commands
docker-build:
	@cd tests/docker && docker-compose build

docker-up:
	@cd tests/docker && docker-compose up -d
	@echo "Waiting for SPICE server to be ready..."
	@sleep 5
	@docker exec spice-test-server nc -z localhost 5900 && echo "SPICE server is ready!" || echo "SPICE server failed to start"

docker-down:
	@cd tests/docker && docker-compose down -v

# Development workflow shortcuts
dev: fmt lint test-unit

ci: fmt lint test-all

# Install development dependencies
install-deps:
	@echo "Installing development dependencies..."
	@cargo install cargo-tarpaulin
	@cargo install wasm-pack
	@cargo install cargo-watch
	@echo "Dependencies installed!"

# Watch for changes and run tests
watch:
	@cargo watch -x "test --lib"

# Quick test - runs fast unit tests only
quick:
	@cargo test --lib --all-features -- --test-threads=4

# Run specific test
test-one:
	@if [ -z "$(TEST)" ]; then \
		echo "Usage: make test-one TEST=test_name"; \
	else \
		cargo test $(TEST) -- --nocapture; \
	fi

# Generate coverage report
coverage:
	@cargo tarpaulin --out Html --all-features --ignore-tests
	@echo "Coverage report generated at target/tarpaulin/tarpaulin-report.html"