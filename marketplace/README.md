# Lumaris Marketplace

A decentralized marketplace to buy and sell computational power.

## Components

The Lumaris Marketplace consists of the following components:

1. **FastAPI Backend** - Handles API requests, database operations, and business logic
2. **Django Admin Dashboard** - Provides a user-friendly interface for monitoring and managing the marketplace
3. **PostgreSQL Database** - Stores all marketplace data
4. **Nginx** - Serves as a reverse proxy and handles SSL termination

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

## Development

For development, you can run the services individually:

1. **API (FastAPI)**:
   ```
   cd api
   python -m venv venv
   source venv/bin/activate  # On Windows: venv\Scripts\activate
   pip install -r requirements.txt
   uvicorn main:app --reload --port 8080
   ```

2. **Admin Dashboard (Django)**:
   ```
   cd admin_dashboard
   python -m venv venv
   source venv/bin/activate  # On Windows: venv\Scripts\activate
   pip install -r requirements.txt
   python manage.py runserver
   ```

## Production Deployment

For production deployment:

1. Set `DEBUG=false` in the `.env` file
2. Use proper SSL certificates
3. Set strong passwords for the database and API key
4. Configure proper ALLOWED_HOSTS and CORS settings

## License

[MIT License](LICENSE)

