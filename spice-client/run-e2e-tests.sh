#!/bin/bash
# Unified E2E test runner for SPICE client
# Usage: ./run-e2e-tests.sh [implementation] [server] [options]

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
IMPLEMENTATION="basic"
SERVER="debug"
TEST_DURATION="30"
CLEAN_AFTER="true"
VERBOSE=""
DRY_RUN=""
HEADLESS=""

# Ensure display variables are defined (may be empty)
WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-}"
DISPLAY="${DISPLAY:-}"
XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-}"

# Help function
show_help() {
    cat << EOF
SPICE Client E2E Test Runner

Usage: $(basename "$0") [implementation] [server] [options]

IMPLEMENTATIONS:
  basic         Basic client without multimedia (default)
  gtk4          GTK4 backend implementation
  wasm-core     WebAssembly core (no browser)
  all           Run all implementations

SERVERS:
  debug         SPICE debug test server (default)
  qemu          QEMU-based SPICE server
  all           Test against all servers

OPTIONS:
  -d, --duration TIME    Test duration in seconds (default: 30)
  -k, --keep            Don't clean up after tests
  -v, --verbose         Enable verbose output
  -n, --dry-run         Show what would be run without executing
  --headless            Run GUI tests in headless mode (default: show UI)
  -h, --help            Show this help message

EXAMPLES:
  # Quick test with defaults
  $(basename "$0")

  # Test basic client against QEMU
  $(basename "$0") basic qemu

  # Test all implementations against debug server
  $(basename "$0") all debug

  # Full matrix test (all implementations × all servers)
  $(basename "$0") all all

  # Verbose test with longer duration
  $(basename "$0") basic debug -v -d 60

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            # Implementations
            basic|gtk4|wasm-core|all)
                IMPLEMENTATION="$1"
                shift
                ;;
            # Servers
            debug|qemu)
                SERVER="$1"
                shift
                ;;
            # Options
            -d|--duration)
                TEST_DURATION="$2"
                shift 2
                ;;
            -k|--keep)
                CLEAN_AFTER="false"
                shift
                ;;
            -v|--verbose)
                VERBOSE="-v"
                shift
                ;;
            -n|--dry-run)
                DRY_RUN="true"
                shift
                ;;
            --headless)
                HEADLESS="true"
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                show_help
                exit 1
                ;;
        esac
    done
}

# Print test configuration
print_config() {
    echo -e "${BLUE}=== SPICE E2E Test Configuration ===${NC}"
    echo "Implementation: $IMPLEMENTATION"
    echo "Server: $SERVER"
    echo "Duration: ${TEST_DURATION}s"
    echo "Clean after: $CLEAN_AFTER"
    echo "Verbose: ${VERBOSE:-false}"
    echo "Headless: ${HEADLESS:-false}"
    echo ""
}

# Run Docker Compose with the correct profiles
run_test() {
    local impl=$1
    local server=$2
    local test_name="${impl}_${server}"
    
    echo -e "${YELLOW}Running test: $test_name${NC}"
    
    # Determine profiles based on implementation and server
    local profiles=""
    local server_profile=""
    local test_profile=""
    
    # Server profile
    case $server in
        debug)
            server_profile="server-debug"
            export SPICE_SERVER="spice-debug-server"
            export SPICE_PORT="5912"
            ;;
        qemu)
            server_profile="server-qemu"
            export SPICE_SERVER="spice-qemu-server"
            export SPICE_PORT="5900"
            ;;
    esac
    
    # Test profile
    case $impl in
        basic)
            test_profile="test-native"
            ;;
        gtk4)
            test_profile="test-gtk4"
            ;;
        wasm-core)
            test_profile="test-wasm-core"
            ;;
    esac
    
    profiles="--profile $server_profile --profile $test_profile"
    
    # Export test configuration
    export TEST_DURATION
    
    # Configure display based on headless mode
    if [[ -n "$HEADLESS" ]]; then
        # Force headless mode
        export DISPLAY=":99"
        unset WAYLAND_DISPLAY
        unset USE_GTK_BINARY
    else
        # For GTK4, use the visual binary unless explicitly testing
        if [[ "$impl" == "gtk4" ]]; then
            export USE_GTK_BINARY="true"
            echo -e "${BLUE}GTK4: Using visual binary (rusty-spice-gtk)${NC}"
        fi
        
        # Pass through host display environment
        if [[ -n "$WAYLAND_DISPLAY" ]]; then
            export WAYLAND_DISPLAY
            export XDG_RUNTIME_DIR
            echo -e "${BLUE}Using Wayland display: $WAYLAND_DISPLAY${NC}"
        elif [[ -n "$DISPLAY" ]]; then
            export DISPLAY
            echo -e "${BLUE}Using X11 display: $DISPLAY${NC}"
        else
            # Fallback to headless if no display available
            export DISPLAY=":99"
            echo -e "${YELLOW}No display found, falling back to headless mode${NC}"
        fi
    fi
    
    # Build command
    local cmd="docker compose -f docker/docker-compose.yml $profiles"
    
    if [[ -n "$DRY_RUN" ]]; then
        echo -e "${BLUE}Would run:${NC} $cmd up --build --abort-on-container-exit"
        return 0
    fi
    
    # Run the test
    if $cmd up --build --abort-on-container-exit --exit-code-from test-$impl; then
        echo -e "${GREEN}✓ Test $test_name passed${NC}"
        
        # Collect logs if verbose
        if [[ -n "$VERBOSE" ]]; then
            echo -e "${BLUE}Collecting logs...${NC}"
            $cmd logs > "test-results/${test_name}.log" 2>&1
        fi
        
        # Clean up if requested
        if [[ "$CLEAN_AFTER" == "true" ]]; then
            $cmd down -v
        fi
        
        return 0
    else
        echo -e "${RED}✗ Test $test_name failed${NC}"
        
        # Always collect logs on failure
        echo -e "${BLUE}Collecting failure logs...${NC}"
        $cmd logs > "test-results/${test_name}_failure.log" 2>&1
        
        # Clean up even on failure
        if [[ "$CLEAN_AFTER" == "true" ]]; then
            $cmd down -v
        fi
        
        return 1
    fi
}

# Main test runner
main() {
    parse_args "$@"
    print_config
    
    # Change to repository root
    cd "$(dirname "$0")"
    
    # Create results directory
    mkdir -p test-results
    
    # Determine which tests to run
    local implementations=()
    local servers=()
    
    case $IMPLEMENTATION in
        all)
            implementations=("basic" "gtk4" "wasm-core")
            ;;
        *)
            implementations=("$IMPLEMENTATION")
            ;;
    esac
    
    case $SERVER in
        all)
            servers=("debug" "qemu")
            ;;
        *)
            servers=("$SERVER")
            ;;
    esac
    
    # Track test results
    local total_tests=0
    local passed_tests=0
    local failed_tests=()
    
    # Run test matrix
    for impl in "${implementations[@]}"; do
        for srv in "${servers[@]}"; do
            total_tests=$((total_tests + 1))
            
            if run_test "$impl" "$srv"; then
                passed_tests=$((passed_tests + 1))
            else
                failed_tests+=("${impl}_${srv}")
            fi
            
            # Small delay between tests
            sleep 2
        done
    done
    
    # Print summary
    echo ""
    echo -e "${BLUE}=== Test Summary ===${NC}"
    echo "Total tests: $total_tests"
    echo -e "${GREEN}Passed: $passed_tests${NC}"
    echo -e "${RED}Failed: ${#failed_tests[@]}${NC}"
    
    if [[ ${#failed_tests[@]} -gt 0 ]]; then
        echo ""
        echo -e "${RED}Failed tests:${NC}"
        for test in "${failed_tests[@]}"; do
            echo "  - $test"
        done
        exit 1
    else
        echo -e "${GREEN}All tests passed!${NC}"
        exit 0
    fi
}

# Run main function
main "$@"