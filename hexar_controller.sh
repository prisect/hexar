#!/bin/bash

# Hexar Radar System - Production Controller Script
# This script provides a production-ready interface for the hexar radar system

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LOG_DIR="$PROJECT_ROOT/logs"
CONFIG_FILE="$PROJECT_ROOT/config.toml"
PID_FILE="$PROJECT_ROOT/hexar.pid"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Ensure log directory exists
mkdir -p "$LOG_DIR"

# Check if Rust/Cargo is available
check_dependencies() {
    if ! command -v cargo &> /dev/null; then
        error "Cargo is not installed or not in PATH"
        error "Please install Rust from https://rustup.rs/"
        exit 1
    fi
    
    if ! command -v wsl &> /dev/null; then
        error "WSL is not available"
        error "This script requires WSL to run the radar system"
        exit 1
    fi
}

# Build the project
build_project() {
    log "Building hexar project..."
    cd "$PROJECT_ROOT"
    
    if wsl bash -c ". ~/.cargo/env && cargo build --release" 2>&1 | tee "$LOG_DIR/build.log"; then
        success "Build completed successfully"
    else
        error "Build failed. Check $LOG_DIR/build.log for details"
        exit 1
    fi
}

# Start the radar system
start_system() {
    local daemon_mode=${1:-false}
    local unsafe_mode=${2:-false}
    
    log "Starting hexar radar system..."
    
    # Check if already running
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        warn "System is already running (PID: $(cat "$PID_FILE"))"
        return 0
    fi
    
    # Build project
    build_project
    
    # Prepare command arguments
    local cmd_args="start"
    if [ "$daemon_mode" = true ]; then
        cmd_args="$cmd_args --daemon"
    fi
    if [ "$unsafe_mode" = true ]; then
        cmd_args="$cmd_args --unsafe-mode"
        warn "Starting in UNSAFE MODE - safety checks bypassed"
    fi
    
    if [ -f "$CONFIG_FILE" ]; then
        cmd_args="$cmd_args --config $CONFIG_FILE"
    fi
    
    # Start the system
    cd "$PROJECT_ROOT"
    log "Executing: wsl bash -c '. ~/.cargo/env && cargo run --release --bin hexar -- $cmd_args'"
    
    if [ "$daemon_mode" = true ]; then
        # Start in background
        nohup wsl bash -c ". ~/.cargo/env && cargo run --release --bin hexar -- $cmd_args" > "$LOG_DIR/hexar.log" 2>&1 &
        echo $! > "$PID_FILE"
        success "System started in daemon mode (PID: $(cat "$PID_FILE"))"
        success "Logs: $LOG_DIR/hexar.log"
    else
        # Start in foreground
        wsl bash -c ". ~/.cargo/env && cargo run --release --bin hexar -- $cmd_args"
    fi
}

# Stop the radar system
stop_system() {
    local timeout=${1:-30}
    
    log "Stopping hexar radar system..."
    
    if [ ! -f "$PID_FILE" ]; then
        warn "PID file not found. System may not be running."
        return 0
    fi
    
    local pid=$(cat "$PID_FILE")
    
    if ! kill -0 "$pid" 2>/dev/null; then
        warn "Process $pid is not running. Removing PID file."
        rm -f "$PID_FILE"
        return 0
    fi
    
    # Send SIGTERM for graceful shutdown
    log "Sending SIGTERM to process $pid..."
    kill -TERM "$pid"
    
    # Wait for graceful shutdown
    local count=0
    while [ $count -lt $timeout ] && kill -0 "$pid" 2>/dev/null; do
        sleep 1
        count=$((count + 1))
    done
    
    # Check if process is still running
    if kill -0 "$pid" 2>/dev/null; then
        warn "Graceful shutdown timed out. Sending SIGKILL..."
        kill -KILL "$pid"
        sleep 2
        
        if kill -0 "$pid" 2>/dev/null; then
            error "Failed to kill process $pid"
            exit 1
        fi
    fi
    
    rm -f "$PID_FILE"
    success "System stopped successfully"
}

# Show system status
show_status() {
    local detailed=${1:-false}
    
    log "Checking system status..."
    
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        local pid=$(cat "$PID_FILE")
        success "System is running (PID: $pid)"
        
        if [ "$detailed" = true ]; then
            log "Detailed status:"
            log "  PID: $pid"
            log "  Start time: $(ps -o lstart= -p "$pid" 2>/dev/null || echo "Unknown")"
            log "  Memory usage: $(ps -o rss= -p "$pid" 2>/dev/null || echo "Unknown") KB"
            log "  CPU usage: $(ps -o %cpu= -p "$pid" 2>/dev/null || echo "Unknown")%"
            log "  Config file: ${CONFIG_FILE:-"Default"}"
            log "  Log directory: $LOG_DIR"
        fi
        
        # Show recent log entries
        if [ -f "$LOG_DIR/hexar.log" ]; then
            log "Recent log entries:"
            tail -n 10 "$LOG_DIR/hexar.log" | sed 's/^/  /'
        fi
    else
        warn "System is not running"
        if [ -f "$PID_FILE" ]; then
            warn "Removing stale PID file"
            rm -f "$PID_FILE"
        fi
    fi
}

# Run diagnostics
run_diagnostics() {
    local component=${1:-""}
    
    log "Running system diagnostics..."
    
    build_project
    
    local cmd_args="diagnose"
    if [ -n "$component" ]; then
        cmd_args="$cmd_args --component $component"
    fi
    
    if [ -f "$CONFIG_FILE" ]; then
        cmd_args="$cmd_args --config $CONFIG_FILE"
    fi
    
    cd "$PROJECT_ROOT"
    wsl bash -c ". ~/.cargo/env && cargo run --release --bin hexar -- $cmd_args"
}

# Configuration management
handle_config() {
    local action=$1
    local key=${2:-""}
    local value=${3:-""}
    
    log "Configuration action: $action"
    
    build_project
    
    local cmd_args="config $action"
    
    case $action in
        "set")
            if [ -z "$key" ] || [ -z "$value" ]; then
                error "Config set requires key and value"
                exit 1
            fi
            cmd_args="$cmd_args $key $value"
            ;;
        "show"|"validate"|"reset")
            # No additional arguments needed
            ;;
        *)
            error "Unknown config action: $action"
            exit 1
            ;;
    esac
    
    if [ -f "$CONFIG_FILE" ]; then
        cmd_args="$cmd_args --config $CONFIG_FILE"
    fi
    
    cd "$PROJECT_ROOT"
    wsl bash -c ". ~/.cargo/env && cargo run --release --bin hexar -- $cmd_args"
}

# Monitor system
monitor_system() {
    local follow=${1:-false}
    local level=${2:-""}
    
    log "Starting system monitoring..."
    
    if [ "$follow" = true ]; then
        if [ -f "$LOG_DIR/hexar.log" ]; then
            log "Following logs (Ctrl+C to stop):"
            tail -f "$LOG_DIR/hexar.log"
        else
            warn "Log file not found: $LOG_DIR/hexar.log"
        fi
    else
        # Show recent logs
        if [ -f "$LOG_DIR/hexar.log" ]; then
            log "Recent log entries:"
            tail -n 50 "$LOG_DIR/hexar.log"
        else
            warn "Log file not found: $LOG_DIR/hexar.log"
        fi
    fi
}

# Show usage information
show_usage() {
    cat << EOF
Hexar Radar System Controller

Usage: $0 COMMAND [OPTIONS]

Commands:
    start [--daemon] [--unsafe]     Start the radar system
    stop [timeout]                   Stop the radar system
    status [--detailed]              Show system status
    diagnose [component]             Run diagnostics
    config <action> [key] [value]   Configuration management
    monitor [--follow] [level]      Monitor system logs
    build                            Build the project
    help                             Show this help

Config Actions:
    show                             Show current configuration
    validate                         Validate configuration
    reset                            Reset to defaults
    set <key> <value>                Set configuration value

Examples:
    $0 start                         Start in foreground mode
    $0 start --daemon                Start in daemon mode
    $0 stop                          Stop gracefully
    $0 status --detailed             Show detailed status
    $0 diagnose                      Run full diagnostics
    $0 config show                   Show configuration
    $0 config set radar.antenna_count 8  Set antenna count
    $0 monitor --follow              Follow logs in real-time

Safety Notes:
- Always run diagnostics before starting the system
- Use --unsafe flag only for testing/debugging
- Monitor logs for any safety alerts
- Ensure proper ventilation and power supply

EOF
}

# Main script logic
main() {
    # Check dependencies
    check_dependencies
    
    # Parse command line arguments
    case "${1:-help}" in
        "start")
            local daemon=false
            local unsafe=false
            
            shift
            while [[ $# -gt 0 ]]; do
                case $1 in
                    "--daemon")
                        daemon=true
                        ;;
                    "--unsafe")
                        unsafe=true
                        ;;
                    *)
                        error "Unknown option: $1"
                        show_usage
                        exit 1
                        ;;
                esac
                shift
            done
            
            start_system "$daemon" "$unsafe"
            ;;
        "stop")
            stop_system "${2:-30}"
            ;;
        "status")
            local detailed=false
            if [[ "${2:-}" == "--detailed" ]]; then
                detailed=true
            fi
            show_status "$detailed"
            ;;
        "diagnose")
            run_diagnostics "${2:-}"
            ;;
        "config")
            handle_config "${2:-}" "${3:-}" "${4:-}"
            ;;
        "monitor")
            local follow=false
            local level=""
            
            if [[ "${2:-}" == "--follow" ]]; then
                follow=true
                level="${3:-}"
            else
                level="${2:-}"
            fi
            
            monitor_system "$follow" "$level"
            ;;
        "build")
            build_project
            ;;
        "help"|"--help"|"-h")
            show_usage
            ;;
        *)
            error "Unknown command: ${1:-}"
            show_usage
            exit 1
            ;;
    esac
}

# Trap signals for cleanup
trap 'warn "Interrupted"; exit 130' INT TERM

# Run main function with all arguments
main "$@"
