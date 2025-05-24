#!/bin/bash

# Build and run script for Lumaris Marketplace

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Lumaris Marketplace Build Script ===${NC}"

# Function to check if a command exists
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# Check for required tools
if ! command_exists cargo; then
  echo -e "${RED}Error: Rust and Cargo are required but not installed.${NC}"
  echo "Please install Rust from https://rustup.rs/"
  exit 1
fi

if ! command_exists python3; then
  echo -e "${RED}Error: Python 3 is required but not installed.${NC}"
  exit 1
fi

# Build all Rust components
build_rust() {
  echo -e "${YELLOW}Building all Rust components...${NC}"
  
  # Build from workspace root
  cargo build --release
  
  echo -e "${GREEN}✓ All Rust components built successfully${NC}"
}

# Run specific components
run_node_ws() {
  echo -e "${YELLOW}Starting Node WebSocket Server...${NC}"
  cargo run --release --bin node_ws
}

run_rust_gui() {
  echo -e "${YELLOW}Starting Node Dashboard GUI...${NC}"
  cargo run --release --bin rust_gui
}

run_admin_api() {
  echo -e "${YELLOW}Starting Admin API...${NC}"
  cd marketplace/admin_api
  python3 -m uvicorn main:app --reload --port 8000
}

# Setup Python environment
setup_python() {
  echo -e "${YELLOW}Setting up Python environment...${NC}"
  
  if command_exists conda; then
    conda env update -f marketplace/environment.yml
  else
    python3 -m pip install -r marketplace/requirements.txt
  fi
  
  echo -e "${GREEN}✓ Python environment setup complete${NC}"
}

# Display help
show_help() {
  echo -e "Usage: $0 [command]"
  echo -e ""
  echo -e "Commands:"
  echo -e "  build         Build all Rust components"
  echo -e "  node-ws       Run the Node WebSocket Server"
  echo -e "  gui           Run the Node Dashboard GUI"
  echo -e "  admin-api     Run the Admin API"
  echo -e "  setup-python  Setup Python environment"
  echo -e "  all           Build and run all components"
  echo -e "  help          Show this help message"
}

# Main execution
case "$1" in
  "build")
    build_rust
    ;;
  "node-ws")
    run_node_ws
    ;;
  "gui")
    run_rust_gui
    ;;
  "admin-api")
    run_admin_api
    ;;
  "setup-python")
    setup_python
    ;;
  "all")
    build_rust
    setup_python
    
    # Run all components in separate terminals if possible
    if command_exists gnome-terminal; then
      gnome-terminal -- bash -c "cd $(pwd) && ./build.sh node-ws; bash"
      gnome-terminal -- bash -c "cd $(pwd) && ./build.sh admin-api; bash"
      ./build.sh gui
    elif command_exists xterm; then
      xterm -e "cd $(pwd) && ./build.sh node-ws; bash" &
      xterm -e "cd $(pwd) && ./build.sh admin-api; bash" &
      ./build.sh gui
    else
      echo -e "${YELLOW}Cannot open multiple terminals automatically.${NC}"
      echo -e "Please run each component separately:"
      echo -e "  ./build.sh node-ws"
      echo -e "  ./build.sh admin-api"
      echo -e "  ./build.sh gui"
    fi
    ;;
  "help"|"")
    show_help
    ;;
  *)
    echo -e "${RED}Unknown command: $1${NC}"
    show_help
    exit 1
    ;;
esac

