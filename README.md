# Lumaris Marketplace

A decentralized marketplace to buy and sell computational power.

## Project Overview

Lumaris Marketplace is a platform that enables users to buy and sell computational resources in a decentralized manner. The platform consists of several components:

- **Marketplace API**: Handles user authentication, job submissions, and payments
- **Node WebSocket Server**: Manages communication with compute nodes
- **Distributed Engine**: Splits jobs into chunks and distributes them to nodes
- **Rust GUI**: Provides a user interface for node operators

## Components

### Marketplace API

The API server handles:
- User registration and authentication
- Job submissions and tracking
- Payment processing
- Node registration and management

### Node WebSocket Server

The WebSocket server:
- Manages connections with compute nodes
- Distributes jobs to available nodes
- Tracks node status and availability
- Collects job results

### Distributed Engine

The distributed engine:
- Splits large jobs into smaller chunks
- Distributes chunks to available nodes
- Aggregates results from multiple nodes
- Handles fault tolerance and retries

### Rust GUI

The GUI application:
- Allows node operators to manage their nodes
- Displays node status and performance metrics
- Provides job history and earnings information
- Configures node capabilities and availability

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Cargo
- PostgreSQL
- Redis (optional)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/BDR-Pro/lumaris_marketplace.git
cd lumaris_marketplace
```

2. Build the project:
```bash
cargo build --release
```

3. Run the components:

**API Server:**
```bash
cd marketplace/api
cargo run --release
```

**Node WebSocket Server:**
```bash
cd marketplace/node_ws
cargo run --release
```

**GUI Application:**
```bash
cd marketplace/rust_gui
cargo run --release
```

## Configuration

Each component has its own configuration file in TOML format. Copy the example configuration files and modify them as needed:

```bash
cp marketplace/api/config.example.toml marketplace/api/config.toml
cp marketplace/node_ws/config.example.toml marketplace/node_ws/config.toml
```

## Development

### Running Tests

```bash
cargo test
```

### Building Documentation

```bash
cargo doc --open
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

