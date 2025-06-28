#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$SCRIPT_DIR/tests/docker"
COMPOSE_FILE="$DOCKER_DIR/docker-compose.yml"
TEST_TIMEOUT=${TEST_TIMEOUT:-300}  # 5 minutes default
SPICE_PORT=${SPICE_PORT:-5900}
CONTAINER_NAME="spice-test-server"
SKIP_CLEANUP=${SKIP_CLEANUP:-false}

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Cleanup function
cleanup() {
    if [ "$SKIP_CLEANUP" != "true" ]; then
        log_info "Cleaning up..."
        docker compose -f "$COMPOSE_FILE" down -v 2>/dev/null || true
        docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
    else
        log_warning "Skipping cleanup (SKIP_CLEANUP=true)"
    fi
}

# Trap cleanup on exit
trap cleanup EXIT

# Check dependencies
check_dependencies() {
    log_info "Checking dependencies..."
    
    local deps=("docker" "cargo")
    local missing=()
    
    for cmd in "${deps[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            missing+=("$cmd")
        fi
    done
    
    if [ ${#missing[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing[*]}"
        log_info "Please install the missing dependencies and try again."
        exit 1
    fi
    
    # Check if Docker daemon is running
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        exit 1
    fi
    
    log_success "All dependencies are installed"
}

# Build Docker image
build_docker_image() {
    log_info "Building SPICE server Docker image..."
    
    cd "$DOCKER_DIR"
    
    # Make start-qemu.sh executable
    chmod +x start-qemu.sh
    
    # Build the image
    if docker compose build --no-cache; then
        log_success "Docker image built successfully"
    else
        log_error "Failed to build Docker image"
        exit 1
    fi
    
    cd - > /dev/null
}

# Start SPICE server
start_spice_server() {
    log_info "Starting SPICE server container..."
    
    # Stop any existing container
    docker compose -f "$COMPOSE_FILE" down 2>/dev/null || true
    
    # Start the container
    if docker compose -f "$COMPOSE_FILE" up -d; then
        log_success "SPICE server container started"
    else
        log_error "Failed to start SPICE server container"
        exit 1
    fi
    
    # Wait for container to be healthy
    log_info "Waiting for SPICE server to be ready..."
    local max_attempts=30
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if docker exec "$CONTAINER_NAME" nc -z localhost "$SPICE_PORT" 2>/dev/null; then
            log_success "SPICE server is ready on port $SPICE_PORT"
            return 0
        fi
        
        attempt=$((attempt + 1))
        echo -n "."
        sleep 2
    done
    
    echo
    log_error "SPICE server failed to become ready"
    
    # Show container logs for debugging
    log_error "Container logs:"
    docker logs "$CONTAINER_NAME" 2>&1 | tail -20
    
    return 1
}

# Run unit tests
run_unit_tests() {
    log_info "Running unit tests..."
    
    cd "$SCRIPT_DIR"
    
    if cargo test --lib --all-features; then
        log_success "Unit tests passed"
    else
        log_error "Unit tests failed"
        return 1
    fi
}

# Run integration tests
run_integration_tests() {
    log_info "Running integration tests..."
    
    cd "$SCRIPT_DIR"
    
    # Set environment variables
    export SPICE_TEST_HOST=localhost
    export SPICE_TEST_PORT=$SPICE_PORT
    export SPICE_INTEGRATION_TESTS=1
    export RUST_LOG=debug
    export RUST_BACKTRACE=1
    
    log_info "Test configuration:"
    log_info "  Host: $SPICE_TEST_HOST"
    log_info "  Port: $SPICE_TEST_PORT"
    
    # Run all integration tests
    if timeout "$TEST_TIMEOUT" cargo test --test '*' -- --nocapture; then
        log_success "Integration tests passed"
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            log_error "Integration tests timed out after ${TEST_TIMEOUT}s"
        else
            log_error "Integration tests failed"
        fi
        
        # Show container logs on failure
        log_warning "SPICE server logs:"
        docker logs "$CONTAINER_NAME" 2>&1 | tail -50
        
        return 1
    fi
}

# Run specific test
run_specific_test() {
    local test_name="$1"
    log_info "Running specific test: $test_name"
    
    cd "$SCRIPT_DIR"
    
    export SPICE_TEST_HOST=localhost
    export SPICE_TEST_PORT=$SPICE_PORT
    export SPICE_INTEGRATION_TESTS=1
    export RUST_LOG=debug
    
    if cargo test "$test_name" -- --nocapture --exact; then
        log_success "Test '$test_name' passed"
    else
        log_error "Test '$test_name' failed"
        return 1
    fi
}

# Show help
show_help() {
    cat << EOF
Usage: $0 [OPTIONS] [TEST_NAME]

Run SPICE client integration tests

OPTIONS:
    -h, --help          Show this help message
    -b, --build-only    Only build Docker image, don't run tests
    -u, --unit-only     Only run unit tests
    -i, --integration-only  Only run integration tests (assumes server is running)
    -s, --skip-cleanup  Don't clean up Docker containers after tests
    -p, --port PORT     SPICE server port (default: 5900)
    -t, --timeout SEC   Test timeout in seconds (default: 300)
    -l, --logs          Show container logs
    -v, --verbose       Enable verbose output

EXAMPLES:
    # Run all tests
    $0

    # Run only unit tests
    $0 --unit-only

    # Run specific test
    $0 test_connect_to_spice_server

    # Run with custom port
    $0 --port 5901

    # Keep containers running after tests
    $0 --skip-cleanup

EOF
}

# Show container logs
show_logs() {
    log_info "Container logs:"
    docker logs "$CONTAINER_NAME" 2>&1
}

# Main function
main() {
    local build_only=false
    local unit_only=false
    local integration_only=false
    local show_logs_flag=false
    local test_name=""
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_help
                exit 0
                ;;
            -b|--build-only)
                build_only=true
                shift
                ;;
            -u|--unit-only)
                unit_only=true
                shift
                ;;
            -i|--integration-only)
                integration_only=true
                shift
                ;;
            -s|--skip-cleanup)
                SKIP_CLEANUP=true
                shift
                ;;
            -p|--port)
                SPICE_PORT="$2"
                shift 2
                ;;
            -t|--timeout)
                TEST_TIMEOUT="$2"
                shift 2
                ;;
            -l|--logs)
                show_logs_flag=true
                shift
                ;;
            -v|--verbose)
                export RUST_LOG=trace
                set -x
                shift
                ;;
            *)
                test_name="$1"
                shift
                ;;
        esac
    done
    
    log_info "SPICE Client Integration Test Runner"
    log_info "===================================="
    
    # Check dependencies
    check_dependencies
    
    # Build Docker image if needed
    if [ "$integration_only" != "true" ]; then
        build_docker_image
        
        if [ "$build_only" = "true" ]; then
            log_success "Docker image built successfully"
            exit 0
        fi
    fi
    
    # Run unit tests if not integration-only
    if [ "$unit_only" = "true" ]; then
        run_unit_tests
        exit $?
    fi
    
    # Start SPICE server if not unit-only
    if [ "$unit_only" != "true" ]; then
        if [ "$integration_only" != "true" ]; then
            if ! start_spice_server; then
                exit 1
            fi
        else
            log_warning "Assuming SPICE server is already running (--integration-only)"
        fi
        
        # Show logs if requested
        if [ "$show_logs_flag" = "true" ]; then
            show_logs
        fi
        
        # Run tests
        if [ -n "$test_name" ]; then
            run_specific_test "$test_name"
        else
            if [ "$integration_only" != "true" ]; then
                run_unit_tests || exit 1
            fi
            run_integration_tests
        fi
    fi
}

# Run main function
main "$@"