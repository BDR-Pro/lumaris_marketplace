# Lumaris Node Dashboard GUI

This component provides a graphical user interface for Lumaris Marketplace seller nodes. It displays real-time system statistics and node earnings.

## Features

- Real-time CPU and memory usage monitoring
- Node earnings display
- Node ID and status information
- WebSocket communication with the central server

## Getting Started

### Prerequisites

- Rust 1.56 or later
- Cargo package manager

### Building

```bash
cargo build --release
```

### Running

```bash
cargo run --release
```

## Architecture

The GUI is built using the egui framework and consists of:

1. **Main Dashboard** - Displays node statistics and earnings
2. **Stats Sender** - Background thread that collects system metrics and sends them to the central server

## Configuration

The GUI connects to the WebSocket server at `ws://127.0.0.1:9001` by default. This can be configured by setting the `LUMARIS_WS_URL` environment variable.

## Customization

The GUI can be customized by modifying the `NodeDashboard` implementation in `main.rs`. Additional panels and visualizations can be added to enhance the user experience.

