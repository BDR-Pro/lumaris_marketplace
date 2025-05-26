"""
Tests for WebSocket heartbeat monitoring functionality.
"""

import asyncio
import pytest
import json
from datetime import datetime, timedelta
from fastapi.testclient import TestClient
from fastapi.websockets import WebSocketDisconnect

from admin_api.main import app
from admin_api.websocket import manager as websocket_manager

# Reset the test client for each test
@pytest.fixture
def client():
    return TestClient(app)

@pytest.fixture
def mock_token():
    # This would normally be a valid JWT token
    return "test_token"

@pytest.fixture
def mock_auth(monkeypatch):
    """Mock the authentication function to always return a valid node."""
    from admin_api.auth import get_current_node_from_token
    
    class MockNode:
        id = "test_node_id"
    
    async def mock_get_current_node(token):
        return MockNode()
    
    monkeypatch.setattr("admin_api.auth.get_current_node_from_token", mock_get_current_node)

@pytest.mark.asyncio
async def test_websocket_heartbeat_timeout(mock_auth, monkeypatch):
    """Test that nodes are disconnected after heartbeat timeout."""
    # Override the heartbeat timeout for testing
    websocket_manager.heartbeat_timeout = 2  # 2 seconds
    websocket_manager.heartbeat_check_interval = 1  # 1 second
    
    # Mock the WebSocket connection
    class MockWebSocket:
        async def accept(self):
            pass
        
        async def send_json(self, data):
            pass
        
        async def close(self, code=1000):
            pass
    
    # Connect a test node
    test_node_id = "test_heartbeat_node"
    await websocket_manager.connect(MockWebSocket(), test_node_id)
    
    # Verify the node is connected
    assert test_node_id in websocket_manager.active_connections
    
    # Wait for the heartbeat timeout
    await asyncio.sleep(3)
    
    # Verify the node was disconnected
    assert test_node_id not in websocket_manager.active_connections
    assert websocket_manager.pool_status["heartbeat_failures"] > 0

@pytest.mark.asyncio
async def test_heartbeat_update_keeps_connection(mock_auth, monkeypatch):
    """Test that updating heartbeats keeps the connection alive."""
    # Override the heartbeat timeout for testing
    websocket_manager.heartbeat_timeout = 2  # 2 seconds
    websocket_manager.heartbeat_check_interval = 1  # 1 second
    
    # Mock the WebSocket connection
    class MockWebSocket:
        async def accept(self):
            pass
        
        async def send_json(self, data):
            pass
        
        async def close(self, code=1000):
            pass
    
    # Connect a test node
    test_node_id = "test_heartbeat_node"
    await websocket_manager.connect(MockWebSocket(), test_node_id)
    
    # Verify the node is connected
    assert test_node_id in websocket_manager.active_connections
    
    # Update the heartbeat
    for _ in range(3):
        await asyncio.sleep(1)
        websocket_manager.update_heartbeat(test_node_id)
    
    # Verify the node is still connected after the timeout period
    assert test_node_id in websocket_manager.active_connections
    
    # Clean up
    websocket_manager.disconnect(test_node_id)

@pytest.mark.asyncio
async def test_connection_stats_tracking(mock_auth):
    """Test that connection statistics are properly tracked."""
    # Reset connection stats
    initial_stats = websocket_manager.get_connection_stats()
    initial_total = initial_stats["total_connections_ever"]
    
    # Mock the WebSocket connection
    class MockWebSocket:
        async def accept(self):
            pass
        
        async def send_json(self, data):
            pass
        
        async def close(self, code=1000):
            pass
    
    # Connect multiple test nodes
    for i in range(3):
        await websocket_manager.connect(MockWebSocket(), f"test_stats_node_{i}")
    
    # Verify connection stats
    stats = websocket_manager.get_connection_stats()
    assert stats["total_connections_ever"] == initial_total + 3
    assert stats["current_connections"] == 3
    
    # Disconnect one node
    websocket_manager.disconnect("test_stats_node_1", reason="error")
    
    # Verify updated stats
    stats = websocket_manager.get_connection_stats()
    assert stats["current_connections"] == 2
    assert stats["connection_errors"] == 1
    
    # Clean up
    for i in range(3):
        if i != 1:  # Skip the already disconnected node
            websocket_manager.disconnect(f"test_stats_node_{i}")

def test_websocket_stats_endpoint(client, mock_auth):
    """Test the WebSocket stats endpoint."""
    response = client.get("/ws/stats")
    assert response.status_code == 200
    stats = response.json()
    
    # Verify the stats structure
    assert "total_connections_ever" in stats
    assert "current_connections" in stats
    assert "max_concurrent_connections" in stats
    assert "connection_errors" in stats
    assert "heartbeat_failures" in stats

