# Lumaris Marketplace

A decentralized marketplace to buy and sell computational power.

## Components

The Lumaris Marketplace consists of the following components:

1. **FastAPI Backend** - Handles API requests, database operations, and business logic
2. **Django Admin Dashboard** - Provides a user-friendly interface for monitoring and managing the marketplace
3. **PostgreSQL Database** - Stores all marketplace data
4. **Nginx** - Serves as a reverse proxy and handles SSL termination
5. **WebSocket Server** - Provides real-time communication between the marketplace and nodes

## Quick Start with Docker Compose

The easiest way to run the entire Lumaris Marketplace is using Docker Compose:

1. Clone the repository:
   ```
   git clone https://github.com/BDR-Pro/lumaris_marketplace.git
   cd lumaris_marketplace
   ```

2. Create a `.env` file based on the example:
   ```
   cp .env.example .env
   ```
   Edit the `.env` file to set your desired configuration.

3. Generate SSL certificates for development (optional):
   ```
   mkdir -p nginx/ssl
   openssl req -x509 -nodes -days 365 -newkey rsa:2048 -keyout nginx/ssl/lumaris.key -out nginx/ssl/lumaris.crt
   ```

4. Start all services:
   ```
   docker-compose up -d
   ```

5. Access the services:
   - Admin Dashboard: https://localhost (or http://localhost:8000 directly)
   - API: https://localhost/api/ (or http://localhost:8080 directly)
   - WebSocket: wss://localhost/ws/ (or ws://localhost:8080/ws directly)

## Development Setup

### Using Conda

For development with Conda, you can create an environment using the provided `environment.yml` file:

```bash
# Create the conda environment
conda env create -f environment.yml

# Activate the environment
conda activate lumaris-marketplace

# Run the tests
cd marketplace
pytest
```

### Using Pip

If you prefer to use pip, you can install the dependencies from the requirements files:

```bash
# Create a virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt
pip install -r requirements-dev.txt

# Run the tests
cd marketplace
pytest
```

## Manual Setup

If you prefer to run the components individually, refer to the README files in each component directory:

- [API README](./api/README.md)
- [Admin Dashboard README](./admin_dashboard/README.md)

## Environment Variables

The following environment variables can be configured in the `.env` file:

| Variable | Description | Default |
|----------|-------------|---------|
| DB_NAME | Database name | lumaris |
| DB_USER | Database username | postgres |
| DB_PASSWORD | Database password | postgres_password |
| DATABASE_URL | Full database connection string | postgresql://postgres:postgres_password@db:5432/lumaris |
| SECRET_KEY | Secret key for Django and FastAPI | your-secret-key-here |
| API_KEY | API key for authentication | your-api-key-here |
| DEBUG | Enable debug mode | true |
| ALLOWED_HOSTS | Comma-separated list of allowed hosts | localhost,127.0.0.1 |
| CORS_ALLOWED_ORIGINS | Comma-separated list of allowed CORS origins | http://localhost:8000,http://127.0.0.1:8000 |
| WS_PORT | WebSocket port | 3030 |

## Architecture

The Lumaris Marketplace follows a microservices architecture:

1. **Frontend (Django Admin Dashboard)**:
   - Provides a user interface for administrators
   - Communicates with the API for data operations
   - Receives real-time updates via WebSockets

2. **Backend (FastAPI)**:
   - Handles API requests
   - Manages database operations
   - Processes business logic
   - Provides WebSocket endpoints for real-time updates

3. **Database (PostgreSQL)**:
   - Stores all marketplace data
   - Used by both frontend and backend

4. **Nginx**:
   - Acts as a reverse proxy
   - Handles SSL termination
   - Routes requests to the appropriate service

5. **WebSocket Server**:
   - Integrated with FastAPI
   - Provides real-time communication
   - Handles node connections, heartbeats, and notifications

## WebSocket API

The WebSocket API provides real-time communication between the marketplace and nodes. It supports the following message types:

### Connection

Connect to the WebSocket server with a valid token:

```
ws://localhost:8080/ws?token=<node_token>
```

### Message Types

1. **Heartbeat**:
   ```json
   {"type": "heartbeat"}
   ```
   Response:
   ```json
   {"type": "heartbeat_ack", "timestamp": "2023-08-01T12:00:00.000000"}
   ```

2. **Status Update**:
   ```json
   {
     "type": "status_update",
     "status": "busy",
     "cpu_usage": 8.5,
     "mem_usage": 12288.0
   }
   ```
   Response:
   ```json
   {"type": "status_update_ack", "timestamp": "2023-08-01T12:00:00.000000"}
   ```

3. **Job Update**:
   ```json
   {
     "type": "job_update",
     "job_id": "job-123",
     "status": "running"
   }
   ```
   Response:
   ```json
   {
     "type": "job_update_ack",
     "job_id": "job-123",
     "timestamp": "2023-08-01T12:00:00.000000"
   }
   ```

## Running Tests

To run the tests, use pytest:

```bash
# Run all tests
pytest

# Run with coverage
pytest --cov=marketplace

# Run specific test file
pytest tests/test_websocket.py

# Run with verbose output
pytest -v
```

## License

[MIT License](LICENSE)

