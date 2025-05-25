# Lumaris Marketplace Tests

This directory contains tests for the Lumaris Marketplace components, including:

- API tests for the admin API
- WebSocket tests for node communication
- Integration tests between components

## Running the Tests

### Python Tests

To run the Python tests for the admin API:

```bash
cd marketplace
pip install -r tests/requirements.txt
pytest tests/
```

### Rust Tests

To run the Rust tests for the node_ws and rust_gui components:

```bash
cd marketplace/node_ws
cargo test

cd marketplace/rust_gui
cargo test
```

## Test Structure

### API Tests

The API tests verify the functionality of the admin API endpoints, including:

- Node registration and management
- Job submission and tracking
- Matchmaking functionality
- Authentication and authorization

### WebSocket Tests

The WebSocket tests verify the functionality of the WebSocket communication, including:

- Connection handling
- Message serialization and deserialization
- Role-based validation
- Heartbeat mechanism
- Reconnection logic

### Integration Tests

The integration tests verify the end-to-end functionality between components, including:

- Node registration and job assignment
- VM creation and management
- Statistics collection and reporting
- Error handling and recovery

## Adding New Tests

When adding new functionality, please add corresponding tests to ensure the functionality works as expected and remains stable over time.

### Adding Python Tests

Add new Python tests to the `tests/` directory, following the existing test structure. Use pytest fixtures for common setup and teardown.

### Adding Rust Tests

Add new Rust tests to the appropriate module's `tests.rs` file. Use the `#[cfg(test)]` attribute to mark test modules and the `#[test]` attribute to mark test functions.

