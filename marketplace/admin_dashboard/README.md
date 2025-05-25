# Lumaris Marketplace Admin Dashboard

A Django-based admin dashboard for monitoring and managing the Lumaris Marketplace.

## Features

- Real-time monitoring of marketplace activity
- Node management
- VM management
- Transaction tracking
- Interactive visualizations with Chart.js
- Real-time updates with WebSockets and HTMX

## Installation

1. Clone the repository
2. Navigate to the admin_dashboard directory
3. Create a virtual environment:
   ```
   python -m venv venv
   ```
4. Activate the virtual environment:
   - Windows: `venv\Scripts\activate`
   - Unix/MacOS: `source venv/bin/activate`
5. Install dependencies:
   ```
   pip install -r requirements.txt
   ```
6. Create a `.env` file with the following variables:
   ```
   SECRET_KEY=your-secret-key
   DEBUG=True
   ALLOWED_HOSTS=localhost,127.0.0.1
   FASTAPI_URL=http://localhost:8000
   API_KEY=your-api-key
   WS_URL=ws://localhost:3030/ws
   ```
7. Run migrations:
   ```
   python manage.py makemigrations
   python manage.py migrate
   ```
8. Create a superuser:
   ```
   python manage.py createsuperuser
   ```
9. Run the development server:
   ```
   python manage.py runserver
   ```

## Usage

1. Access the dashboard at `http://localhost:8000`
2. Log in with your superuser credentials
3. Navigate through the different sections:
   - Dashboard: Overview of marketplace activity
   - Nodes: Manage seller nodes
   - VMs: Manage virtual machines
   - Transactions: Track financial transactions
   - Statistics: View detailed marketplace statistics

## Architecture

- Django web framework
- Chart.js for data visualization
- HTMX for dynamic content updates
- WebSockets for real-time data
- Bootstrap for responsive UI

## API Integration

The dashboard integrates with the Lumaris Marketplace API to:
- Fetch real-time data
- Manage nodes and VMs
- Process transactions
- Monitor marketplace activity

## WebSocket Integration

Real-time updates are provided through WebSockets, allowing for:
- Instant notification of new nodes
- VM status changes
- Transaction processing
- Resource utilization updates

