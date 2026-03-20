#!/bin/bash

# TinyClaw Startup Script

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
CONFIG_PATH=""
LOG_DIR="${HOME}/.local/share/tiny_claw/logs"
DATA_DIR="${HOME}/.local/share/tiny_claw"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -c|--config)
            CONFIG_PATH="$2"
            shift 2
            ;;
        -l|--log-dir)
            LOG_DIR="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -c, --config PATH    Configuration file path"
            echo "  -l, --log-dir PATH   Log directory path"
            echo "  -h, --help          Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Check if binary exists
BINARY_PATH="${HOME}/.cargo/bin/tiny_claw"
if [ ! -f "$BINARY_PATH" ]; then
    # Try to find in current directory
    BINARY_PATH="./target/release/tiny_claw"
    if [ ! -f "$BINARY_PATH" ]; then
        echo -e "${RED}Error: tiny_claw binary not found${NC}"
        echo "Please build the project first: cargo build --release"
        exit 1
    fi
fi

# Create directories
mkdir -p "$LOG_DIR"
mkdir -p "$DATA_DIR"

echo -e "${GREEN}Starting TinyClaw...${NC}"
echo "Log directory: $LOG_DIR"
echo "Data directory: $DATA_DIR"

# Set environment variables
export RUST_LOG=info
export TINY_CLAW_LOG_DIR="$LOG_DIR"
export TINY_CLAW_DATA_DIR="$DATA_DIR"

# Start the application
if [ -n "$CONFIG_PATH" ]; then
    exec "$BINARY_PATH" --config "$CONFIG_PATH"
else
    exec "$BINARY_PATH"
fi
