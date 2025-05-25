"""Integration tests for the Admin API with Rust nodes."""

import pytest
import json
import asyncio
import websockets
import requests
from datetime import datetime, timedelta
from uuid import uuid4

from admin_api.main import create_app
from fastapi.testclient import TestClient

# Create a test client
app = create_app()
client = TestClient(app)


class TestApiIntegration:
    """Test the integration between the Admin API and Rust nodes."""

    def test_health_check(self):
        """Test the health check endpoint."""
        response = client.get("/health")
        assert response.status_code == 200
        assert response.json() == {"status": "healthy"}

    def test_node_registration(self):
        """Test node registration."""
        # Register a node
        node_id = f"test-node-{uuid4()}"
        response = client.post(
            "/nodes/",
            json={
                "node_id": node_id,
                "cpu": 4.0,
                "mem": 8192.0,
                "funds": 100.0,
            },
        )
        assert response.status_code == 200
        assert response.json()["hostname"] == node_id
        assert response.json()["status"] == "online"

        # Verify the node is in the list
        response = client.get("/nodes/")
        assert response.status_code == 200
        nodes = response.json()
        assert any(node["hostname"] == node_id for node in nodes)

    def test_node_token_issuance(self):
        """Test node token issuance."""
        # Register a node
        node_id = f"test-node-{uuid4()}"
        client.post(
            "/nodes/",
            json={
                "node_id": node_id,
                "cpu": 4.0,
                "mem": 8192.0,
                "funds": 100.0,
            },
        )

        # Issue a token
        response = client.post(f"/nodes/token/{node_id}")
        assert response.status_code == 200
        assert "token" in response.json()
        assert "expires_at" in response.json()

    def test_node_stats_update(self):
        """Test node stats update."""
        # Register a node
        node_id = f"test-node-{uuid4()}"
        client.post(
            "/nodes/",
            json={
                "node_id": node_id,
                "cpu": 4.0,
                "mem": 8192.0,
                "funds": 100.0,
            },
        )

        # Update stats
        response = client.post(
            "/nodes/update",
            json={
                "node_id": node_id,
                "cpu": 8.0,
                "mem": 16384.0,
                "funds": 200.0,
            },
        )
        assert response.status_code == 200
        assert response.json()["hostname"] == node_id
        assert response.json()["cpu_usage"] == 8.0
        assert response.json()["mem_usage"] == 16384.0

    def test_job_submission(self):
        """Test job submission."""
        # Register a node
        node_id = f"test-node-{uuid4()}"
        response = client.post(
            "/nodes/",
            json={
                "node_id": node_id,
                "cpu": 4.0,
                "mem": 8192.0,
                "funds": 100.0,
            },
        )
        node_db_id = response.json()["id"]

        # Submit a job
        response = client.post(
            "/jobs/",
            json={
                "node_id": node_db_id,
                "status": "pending",
                "duration": 3600.0,
                "cost": 10.0,
            },
        )
        assert response.status_code == 200
        assert response.json()["node_id"] == node_db_id
        assert response.json()["status"] == "pending"
        assert response.json()["duration"] == 3600.0
        assert response.json()["cost"] == 10.0

        # Verify the job is in the list
        response = client.get("/jobs/")
        assert response.status_code == 200
        jobs = response.json()
        assert any(job["node_id"] == node_db_id for job in jobs)

    def test_matchmaking(self):
        """Test matchmaking functionality."""
        # Create a node capability
        node_id = f"test-node-{uuid4()}"
        response = client.post(
            "/matchmaking/node/update_availability",
            headers={"X-API-Key": "super-secret"},
            json={
                "node_id": node_id,
                "cpu_available": 4.0,
                "mem_available": 8192,
                "status": "available",
            },
        )
        assert response.status_code == 200
        assert response.json()["success"] is True

        # Assign a job
        job_id = f"test-job-{uuid4()}"
        response = client.post(
            "/matchmaking/job/assign",
            headers={"X-API-Key": "super-secret"},
            json={
                "job_id": job_id,
                "node_id": node_id,
            },
        )
        assert response.status_code == 200
        assert response.json()["assigned"] is True

        # Update job status
        response = client.post(
            "/matchmaking/job/status",
            headers={"X-API-Key": "super-secret"},
            json={
                "job_id": job_id,
                "status": "running",
                "progress": 0.5,
                "cpu_time_sec": 60,
                "peak_memory_mb": 1024,
            },
        )
        assert response.status_code == 200
        assert response.json()["status_updated"] is True


class TestWebSocketIntegration:
    """Test the WebSocket integration with Rust nodes."""

    @pytest.mark.asyncio
    async def test_websocket_connection(self):
        """Test WebSocket connection."""
        # Start WebSocket server
        server = await self._start_test_ws_server()
        server_port = server.sockets[0].getsockname()[1]

        # Connect as a client
        uri = f"ws://localhost:{server_port}/ws"
        async with websockets.connect(uri) as websocket:
            # Send identification message
            await websocket.send(
                json.dumps(
                    {
                        "type": "identify",
                        "request_id": str(uuid4()),
                        "role": "seller",
                        "data": {
                            "user_id": "test-seller",
                            "client_version": "0.1.0",
                        },
                        "timestamp": datetime.utcnow().isoformat(),
                    }
                )
            )

            # Wait for response
            response = await websocket.recv()
            response_data = json.loads(response)
            assert response_data["type"] == "identify_ack"
            assert response_data["data"]["success"] is True

        # Clean up
        server.close()
        await server.wait_closed()

    @pytest.mark.asyncio
    async def test_node_registration_via_websocket(self):
        """Test node registration via WebSocket."""
        # Start WebSocket server
        server = await self._start_test_ws_server()
        server_port = server.sockets[0].getsockname()[1]

        # Connect as a client
        uri = f"ws://localhost:{server_port}/ws"
        async with websockets.connect(uri) as websocket:
            # Send identification message
            await websocket.send(
                json.dumps(
                    {
                        "type": "identify",
                        "request_id": str(uuid4()),
                        "role": "seller",
                        "data": {
                            "user_id": "test-seller",
                            "client_version": "0.1.0",
                        },
                        "timestamp": datetime.utcnow().isoformat(),
                    }
                )
            )

            # Wait for response
            await websocket.recv()

            # Send node registration message
            node_id = f"test-node-{uuid4()}"
            await websocket.send(
                json.dumps(
                    {
                        "type": "node_registration",
                        "request_id": str(uuid4()),
                        "role": "seller",
                        "data": {
                            "seller_id": "test-seller",
                            "capabilities": {
                                "cpu_cores": 4.0,
                                "memory_mb": 8192,
                            },
                        },
                        "timestamp": datetime.utcnow().isoformat(),
                    }
                )
            )

            # Wait for response
            response = await websocket.recv()
            response_data = json.loads(response)
            assert response_data["type"] == "node_registration_ack"
            assert response_data["data"]["success"] is True

        # Clean up
        server.close()
        await server.wait_closed()

    @pytest.mark.asyncio
    async def test_vm_creation_via_websocket(self):
        """Test VM creation via WebSocket."""
        # Start WebSocket server
        server = await self._start_test_ws_server()
        server_port = server.sockets[0].getsockname()[1]

        # Connect as a client
        uri = f"ws://localhost:{server_port}/ws"
        async with websockets.connect(uri) as websocket:
            # Send identification message
            await websocket.send(
                json.dumps(
                    {
                        "type": "identify",
                        "request_id": str(uuid4()),
                        "role": "buyer",
                        "data": {
                            "user_id": "test-buyer",
                            "client_version": "0.1.0",
                        },
                        "timestamp": datetime.utcnow().isoformat(),
                    }
                )
            )

            # Wait for response
            await websocket.recv()

            # Send VM creation message
            await websocket.send(
                json.dumps(
                    {
                        "type": "create_vm",
                        "request_id": str(uuid4()),
                        "role": "buyer",
                        "data": {
                            "job_id": 123,
                            "buyer_id": "test-buyer",
                            "vcpu_count": 2,
                            "mem_size_mib": 1024,
                        },
                        "timestamp": datetime.utcnow().isoformat(),
                    }
                )
            )

            # Wait for response
            response = await websocket.recv()
            response_data = json.loads(response)
            assert response_data["type"] == "create_vm_ack"
            assert response_data["data"]["success"] is True

        # Clean up
        server.close()
        await server.wait_closed()

    @pytest.mark.asyncio
    async def test_heartbeat(self):
        """Test heartbeat mechanism."""
        # Start WebSocket server
        server = await self._start_test_ws_server()
        server_port = server.sockets[0].getsockname()[1]

        # Connect as a client
        uri = f"ws://localhost:{server_port}/ws"
        async with websockets.connect(uri) as websocket:
            # Send identification message
            await websocket.send(
                json.dumps(
                    {
                        "type": "identify",
                        "request_id": str(uuid4()),
                        "role": "buyer",
                        "data": {
                            "user_id": "test-buyer",
                            "client_version": "0.1.0",
                        },
                        "timestamp": datetime.utcnow().isoformat(),
                    }
                )
            )

            # Wait for response
            await websocket.recv()

            # Send heartbeat message
            await websocket.send(
                json.dumps(
                    {
                        "type": "heartbeat",
                        "request_id": str(uuid4()),
                        "role": "buyer",
                        "data": {
                            "timestamp": datetime.utcnow().isoformat(),
                        },
                        "timestamp": datetime.utcnow().isoformat(),
                    }
                )
            )

            # Wait for response
            response = await websocket.recv()
            response_data = json.loads(response)
            assert response_data["type"] == "heartbeat_ack"

        # Clean up
        server.close()
        await server.wait_closed()

    async def _start_test_ws_server(self):
        """Start a test WebSocket server."""
        # Define handler for WebSocket connections
        connected_clients = set()

        async def handler(websocket, path):
            connected_clients.add(websocket)
            try:
                async for message in websocket:
                    data = json.loads(message)
                    message_type = data.get("type")

                    # Handle different message types
                    if message_type == "identify":
                        await websocket.send(
                            json.dumps(
                                {
                                    "type": "identify_ack",
                                    "request_id": data["request_id"],
                                    "role": "server",
                                    "data": {"success": True},
                                    "timestamp": datetime.utcnow().isoformat(),
                                }
                            )
                        )
                    elif message_type == "node_registration":
                        await websocket.send(
                            json.dumps(
                                {
                                    "type": "node_registration_ack",
                                    "request_id": data["request_id"],
                                    "role": "server",
                                    "data": {"success": True, "node_id": str(uuid4())},
                                    "timestamp": datetime.utcnow().isoformat(),
                                }
                            )
                        )
                    elif message_type == "create_vm":
                        await websocket.send(
                            json.dumps(
                                {
                                    "type": "create_vm_ack",
                                    "request_id": data["request_id"],
                                    "role": "server",
                                    "data": {
                                        "success": True,
                                        "vm_id": str(uuid4()),
                                        "job_id": data["data"]["job_id"],
                                    },
                                    "timestamp": datetime.utcnow().isoformat(),
                                }
                            )
                        )
                    elif message_type == "heartbeat":
                        await websocket.send(
                            json.dumps(
                                {
                                    "type": "heartbeat_ack",
                                    "request_id": data["request_id"],
                                    "role": "server",
                                    "data": {
                                        "timestamp": datetime.utcnow().isoformat(),
                                    },
                                    "timestamp": datetime.utcnow().isoformat(),
                                }
                            )
                        )
            finally:
                connected_clients.remove(websocket)

        # Start server
        return await websockets.serve(handler, "localhost", 0)


class TestEndToEndIntegration:
    """Test end-to-end integration between Admin API and Rust nodes."""

    def test_node_registration_and_job_assignment(self):
        """Test node registration and job assignment end-to-end."""
        # Register a node
        node_id = f"test-node-{uuid4()}"
        response = client.post(
            "/nodes/",
            json={
                "node_id": node_id,
                "cpu": 4.0,
                "mem": 8192.0,
                "funds": 100.0,
            },
        )
        assert response.status_code == 200
        node_db_id = response.json()["id"]

        # Update node capabilities
        response = client.post(
            "/matchmaking/node/update_availability",
            headers={"X-API-Key": "super-secret"},
            json={
                "node_id": node_id,
                "cpu_available": 4.0,
                "mem_available": 8192,
                "status": "available",
            },
        )
        assert response.status_code == 200

        # Submit a job
        response = client.post(
            "/jobs/",
            json={
                "node_id": node_db_id,
                "status": "pending",
                "duration": 3600.0,
                "cost": 10.0,
            },
        )
        assert response.status_code == 200
        job_id = response.json()["id"]

        # Assign the job to the node
        response = client.post(
            "/matchmaking/job/assign",
            headers={"X-API-Key": "super-secret"},
            json={
                "job_id": str(job_id),
                "node_id": node_id,
            },
        )
        assert response.status_code == 200
        assert response.json()["assigned"] is True

        # Update job status
        response = client.post(
            "/matchmaking/job/status",
            headers={"X-API-Key": "super-secret"},
            json={
                "job_id": str(job_id),
                "status": "running",
                "progress": 0.5,
            },
        )
        assert response.status_code == 200
        assert response.json()["status_updated"] is True

        # Complete the job
        response = client.post(
            "/matchmaking/job/status",
            headers={"X-API-Key": "super-secret"},
            json={
                "job_id": str(job_id),
                "status": "completed",
                "progress": 1.0,
            },
        )
        assert response.status_code == 200
        assert response.json()["status_updated"] is True

