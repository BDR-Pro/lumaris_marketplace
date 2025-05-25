# Lumaris Marketplace Node WebSocket Server

This component handles WebSocket connections from compute nodes in the Lumaris Marketplace.

## Configuration

The WebSocket server can be configured using environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `API_URL` | URL of the Lumaris API server | `http://localhost:8000` |
| `LOG_LEVEL` | Log level (info, debug, warn, error) | `info` |

## Security Considerations

For production deployments, always use HTTPS for the API URL:

```
export API_URL=https://api.yourdomain.com
```

## Running the Server

```bash
# Set environment variables
export API_URL=https://api.yourdomain.com

# Run the server
cargo run --release
```

## Development

For local development, you can use the default configuration:

```bash
cargo run
```

This will use the default API URL (`http://localhost:8000`) and start the WebSocket server on port 3030.

## Docker

You can also run the server using Docker:

```bash
docker build -t lumaris-node-ws .
docker run -p 3030:3030 -e API_URL=https://api.yourdomain.com lumaris-node-ws
```

