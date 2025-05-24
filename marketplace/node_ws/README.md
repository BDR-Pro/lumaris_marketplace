# Lumaris Node WebSocket Server

This component provides the WebSocket server for Lumaris Marketplace node connections. It handles communication between seller nodes and the central marketplace.

## Features

- WebSocket server for real-time node communication
- REST API for job submission and management
- Matchmaking system for assigning jobs to nodes
- Job scheduling and status tracking

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

The server will start on:
- WebSocket: `ws://127.0.0.1:9001`
- REST API: `http://127.0.0.1:9002`

## API Endpoints

### WebSocket API

Nodes connect to the WebSocket server and send/receive messages in JSON format:

#### Node Stats Update
```json
{
  "type": "stats",
  "node_id": "node-123",
  "cpu": 25.5,
  "mem": 40.2
}
```

#### Availability Update
```json
{
  "type": "availability_update",
  "node_id": "node-123",
  "cpu_available": 3.5,
  "mem_available": 2048,
  "status": "ready",
  "reliability_score": 0.98
}
```

#### Job Status Update
```json
{
  "type": "job_status",
  "job_id": 12345,
  "status": "running"
}
```

### REST API

#### Submit Job
- `POST /jobs`
```json
{
  "command": "process_data",
  "args": ["--input", "file.csv"],
  "input_data": "optional raw data",
  "env_vars": {
    "DEBUG": "true"
  },
  "priority": 2,
  "user_id": "user-456"
}
```

#### Get Job Status
- `GET /jobs/{job_id}`

#### List All Jobs
- `GET /jobs`

#### Cancel Job
- `POST /jobs/{job_id}/cancel`

#### Health Check
- `GET /health`

## Architecture

The WebSocket server integrates with the matchmaker and job scheduler to:
1. Track node availability and capabilities
2. Match jobs to suitable nodes
3. Dispatch jobs to nodes
4. Monitor job execution status
5. Report results back to clients

## Error Handling

The server uses a custom error type system for consistent error handling across components.

