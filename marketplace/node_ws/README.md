# Lumaris Marketplace Node WebSocket Server

This component handles WebSocket connections from compute nodes in the Lumaris Marketplace and manages VM instances for buyers using Firecracker.

## Configuration

The WebSocket server can be configured using environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `API_URL` | URL of the Lumaris API server | `http://localhost:8000` |
| `VM_BASE_PATH` | Base path for VM files | `/tmp/lumaris/vms` |
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
export VM_BASE_PATH=/var/lib/lumaris/vms

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
docker run -p 3030:3030 -e API_URL=https://api.yourdomain.com -e VM_BASE_PATH=/var/lib/lumaris/vms lumaris-node-ws
```

## VM Management with Firecracker

This component includes a VM manager that uses Firecracker to create and manage lightweight VMs for buyers. Firecracker is a lightweight virtualization technology developed by AWS that's perfect for this use case.

### VM Lifecycle

1. **Creation**: When a buyer requests computational resources, a VM is created with the specified CPU and memory resources.
2. **Running**: The VM runs the buyer's workload.
3. **Stopping**: The VM can be stopped when not in use to save resources.
4. **Termination**: The VM is terminated when the buyer no longer needs it.

### WebSocket API for VM Management

The WebSocket server provides the following API endpoints for VM management:

#### Create VM

```json
{
  "type": "create_vm",
  "job_id": 123,
  "buyer_id": "buyer-123",
  "vcpu_count": 2,
  "mem_size_mib": 1024
}
```

#### Stop VM

```json
{
  "type": "stop_vm",
  "vm_id": "vm-123"
}
```

#### Terminate VM

```json
{
  "type": "terminate_vm",
  "vm_id": "vm-123"
}
```

#### Get VM Status

```json
{
  "type": "get_vm_status",
  "vm_id": "vm-123"
}
```

#### Get Buyer VMs

```json
{
  "type": "get_buyer_vms",
  "buyer_id": "buyer-123"
}
```

### Firecracker Requirements

To use Firecracker, you need:

1. A Linux kernel image
2. A root filesystem image
3. Firecracker binary

For development, you can use the provided dummy images. For production, you should create proper images with the required software.

