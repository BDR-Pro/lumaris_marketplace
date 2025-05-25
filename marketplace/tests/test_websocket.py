import pytest
import asyncio
import uuid
from fastapi import FastAPI
from fastapi.testclient import TestClient
from fastapi.websockets import WebSocketDisconnect
import json
from unittest.mock import patch, MagicMock

from marketplace.admin_api.main import app
from marketplace.admin_api.models import Node
from marketplace.admin_api.auth import create_node_token

# This is a proper WebSocket test using pytest-asyncio
@pytest.mark.asyncio
class TestWebSocketIntegration:
    
    async def test_websocket_connection(self, test_db):
        """Test that a node can connect to the WebSocket with a valid token."""
        # Create a test node
        hostname = f"test-ws-node-{uuid.uuid4()}"
        node = Node(hostname=hostname, status="online", cpu_usage=4.0, mem_usage=8192.0)
        test_db.add(node)
        test_db.commit()
        test_db.refresh(node)
        
        # Create a token for the node
        token = create_node_token(node.id)
        
        # Use TestClient in async context
        client = TestClient(app)
        
        # Connect to the WebSocket with the token
        with client.websocket_connect(f"/ws?token={token}") as websocket:
            # Expect a connection established message
            data = websocket.receive_json()
            assert data["type"] == "connection_established"
            assert data["node_id"] == node.id
            
            # Send a heartbeat message
            websocket.send_json({"type": "heartbeat"})
            
            # Expect a heartbeat acknowledgment
            response = websocket.receive_json()
            assert response["type"] == "heartbeat_ack"
    
    async def test_invalid_token(self, test_db):
        """Test that a node cannot connect with an invalid token."""
        # Use TestClient in async context
        client = TestClient(app)
        
        # Try to connect with an invalid token
        with pytest.raises(WebSocketDisconnect) as excinfo:
            with client.websocket_connect("/ws?token=invalid_token"):
                pass
        
        # Verify the connection was closed with the correct code
        assert excinfo.value.code == 1008  # Policy violation
    
    async def test_status_update(self, test_db):
        """Test that a node can send status updates."""
        # Create a test node
        hostname = f"test-ws-node-{uuid.uuid4()}"
        node = Node(hostname=hostname, status="online", cpu_usage=4.0, mem_usage=8192.0)
        test_db.add(node)
        test_db.commit()
        test_db.refresh(node)
        
        # Create a token for the node
        token = create_node_token(node.id)
        
        # Use TestClient in async context
        client = TestClient(app)
        
        # Connect to the WebSocket with the token
        with client.websocket_connect(f"/ws?token={token}") as websocket:
            # Skip the connection established message
            websocket.receive_json()
            
            # Send a status update
            websocket.send_json({
                "type": "status_update",
                "status": "busy",
                "cpu_usage": 8.5,
                "mem_usage": 12288.0
            })
            
            # Expect a status update acknowledgment
            response = websocket.receive_json()
            assert response["type"] == "status_update_ack"
    
    async def test_job_update(self, test_db):
        """Test that a node can send job updates."""
        # Create a test node
        hostname = f"test-ws-node-{uuid.uuid4()}"
        node = Node(hostname=hostname, status="online", cpu_usage=4.0, mem_usage=8192.0)
        test_db.add(node)
        test_db.commit()
        test_db.refresh(node)
        
        # Create a token for the node
        token = create_node_token(node.id)
        
        # Use TestClient in async context
        client = TestClient(app)
        
        # Connect to the WebSocket with the token
        with client.websocket_connect(f"/ws?token={token}") as websocket:
            # Skip the connection established message
            websocket.receive_json()
            
            # Send a job update
            job_id = str(uuid.uuid4())
            websocket.send_json({
                "type": "job_update",
                "job_id": job_id,
                "status": "running"
            })
            
            # Expect a job update acknowledgment
            response = websocket.receive_json()
            assert response["type"] == "job_update_ack"
            assert response["job_id"] == job_id
    
    async def test_invalid_message(self, test_db):
        """Test handling of invalid messages."""
        # Create a test node
        hostname = f"test-ws-node-{uuid.uuid4()}"
        node = Node(hostname=hostname, status="online", cpu_usage=4.0, mem_usage=8192.0)
        test_db.add(node)
        test_db.commit()
        test_db.refresh(node)
        
        # Create a token for the node
        token = create_node_token(node.id)
        
        # Use TestClient in async context
        client = TestClient(app)
        
        # Connect to the WebSocket with the token
        with client.websocket_connect(f"/ws?token={token}") as websocket:
            # Skip the connection established message
            websocket.receive_json()
            
            # Send an invalid JSON message
            websocket.send_text("This is not JSON")
            
            # Expect an error response
            response = websocket.receive_json()
            assert response["type"] == "error"
            assert "Invalid JSON" in response["error"]
    
    async def test_unknown_message_type(self, test_db):
        """Test handling of unknown message types."""
        # Create a test node
        hostname = f"test-ws-node-{uuid.uuid4()}"
        node = Node(hostname=hostname, status="online", cpu_usage=4.0, mem_usage=8192.0)
        test_db.add(node)
        test_db.commit()
        test_db.refresh(node)
        
        # Create a token for the node
        token = create_node_token(node.id)
        
        # Use TestClient in async context
        client = TestClient(app)
        
        # Connect to the WebSocket with the token
        with client.websocket_connect(f"/ws?token={token}") as websocket:
            # Skip the connection established message
            websocket.receive_json()
            
            # Send a message with an unknown type
            websocket.send_json({
                "type": "unknown_type",
                "data": "test"
            })
            
            # Expect an error response
            response = websocket.receive_json()
            assert response["type"] == "error"
            assert "Unknown message type" in response["error"]

