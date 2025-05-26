from fastapi import WebSocket, WebSocketDisconnect, Depends, HTTPException, status
from typing import Dict, List, Optional, Any
import json
import logging
from datetime import datetime

from .auth import get_current_node_from_token
from .models import Node
from .metrics import increment_websocket_connections, decrement_websocket_connections

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Store active connections
class ConnectionManager:
    def __init__(self):
        # Map of node_id to WebSocket connection
        self.active_connections: Dict[str, WebSocket] = {}
        # Last heartbeat time for each node
        self.last_heartbeat: Dict[str, datetime] = {}
    
    async def connect(self, websocket: WebSocket, node_id: str):
        await websocket.accept()
        self.active_connections[node_id] = websocket
        self.last_heartbeat[node_id] = datetime.now()
        logger.info(f"Node {node_id} connected via WebSocket")
        increment_websocket_connections()
    
    def disconnect(self, node_id: str):
        if node_id in self.active_connections:
            del self.active_connections[node_id]
        if node_id in self.last_heartbeat:
            del self.last_heartbeat[node_id]
        logger.info(f"Node {node_id} disconnected from WebSocket")
        decrement_websocket_connections()
    
    async def send_message(self, node_id: str, message: Dict[str, Any]):
        if node_id in self.active_connections:
            websocket = self.active_connections[node_id]
            await websocket.send_json(message)
            logger.info(f"Message sent to node {node_id}: {message}")
            return True
        logger.warning(f"Attempted to send message to disconnected node {node_id}")
        return False
    
    async def broadcast(self, message: Dict[str, Any]):
        for node_id, websocket in self.active_connections.items():
            await websocket.send_json(message)
        logger.info(f"Broadcast message sent to {len(self.active_connections)} nodes: {message}")
    
    def update_heartbeat(self, node_id: str):
        if node_id in self.active_connections:
            self.last_heartbeat[node_id] = datetime.now()
            logger.debug(f"Heartbeat updated for node {node_id}")
    
    def get_connected_nodes(self) -> List[str]:
        return list(self.active_connections.keys())
    
    def is_connected(self, node_id: str) -> bool:
        return node_id in self.active_connections


# Create a global connection manager
manager = ConnectionManager()


async def handle_websocket(websocket: WebSocket, token: str):
    """
    Handle WebSocket connections from nodes.
    
    Args:
        websocket: The WebSocket connection
        token: The authentication token
    """
    # Authenticate the node
    try:
        node = get_current_node_from_token(token)
        if not node:
            await websocket.close(code=status.WS_1008_POLICY_VIOLATION)
            return
    except HTTPException:
        await websocket.close(code=status.WS_1008_POLICY_VIOLATION)
        return
    
    # Accept the connection
    await manager.connect(websocket, node.id)
    
    try:
        # Send initial connection confirmation
        await websocket.send_json({
            "type": "connection_established",
            "node_id": node.id,
            "timestamp": datetime.now().isoformat()
        })
        
        # Handle messages
        while True:
            # Wait for messages from the node
            data = await websocket.receive_text()
            
            try:
                message = json.loads(data)
                message_type = message.get("type")
                
                # Handle different message types
                if message_type == "heartbeat":
                    # Update heartbeat timestamp
                    manager.update_heartbeat(node.id)
                    await websocket.send_json({
                        "type": "heartbeat_ack",
                        "timestamp": datetime.now().isoformat()
                    })
                
                elif message_type == "status_update":
                    # Handle node status updates
                    status = message.get("status")
                    cpu_usage = message.get("cpu_usage")
                    mem_usage = message.get("mem_usage")
                    
                    # Update node status in database (would be implemented elsewhere)
                    logger.info(f"Status update from node {node.id}: {status}, CPU: {cpu_usage}, Mem: {mem_usage}")
                    
                    # Acknowledge the update
                    await websocket.send_json({
                        "type": "status_update_ack",
                        "timestamp": datetime.now().isoformat()
                    })
                
                elif message_type == "job_update":
                    # Handle job status updates
                    job_id = message.get("job_id")
                    job_status = message.get("status")
                    
                    # Update job status in database (would be implemented elsewhere)
                    logger.info(f"Job update from node {node.id}: Job {job_id} status: {job_status}")
                    
                    # Acknowledge the update
                    await websocket.send_json({
                        "type": "job_update_ack",
                        "job_id": job_id,
                        "timestamp": datetime.now().isoformat()
                    })
                
                else:
                    # Handle unknown message types
                    logger.warning(f"Unknown message type from node {node.id}: {message_type}")
                    await websocket.send_json({
                        "type": "error",
                        "error": "Unknown message type",
                        "timestamp": datetime.now().isoformat()
                    })
            
            except json.JSONDecodeError:
                logger.error(f"Invalid JSON received from node {node.id}")
                await websocket.send_json({
                    "type": "error",
                    "error": "Invalid JSON",
                    "timestamp": datetime.now().isoformat()
                })
    
    except WebSocketDisconnect:
        # Handle disconnection
        manager.disconnect(node.id)
    except Exception as e:
        # Handle other exceptions
        logger.error(f"WebSocket error for node {node.id}: {str(e)}")
        manager.disconnect(node.id)


async def send_job_notification(node_id: str, job_id: str, job_details: Dict[str, Any]) -> bool:
    """
    Send a job notification to a node.
    
    Args:
        node_id: The ID of the node to notify
        job_id: The ID of the job
        job_details: Details about the job
    
    Returns:
        bool: True if the message was sent, False otherwise
    """
    message = {
        "type": "job_notification",
        "job_id": job_id,
        "details": job_details,
        "timestamp": datetime.now().isoformat()
    }
    return await manager.send_message(node_id, message)


async def broadcast_marketplace_update(update_type: str, details: Dict[str, Any]):
    """
    Broadcast a marketplace update to all connected nodes.
    
    Args:
        update_type: The type of update
        details: Details about the update
    """
    message = {
        "type": "marketplace_update",
        "update_type": update_type,
        "details": details,
        "timestamp": datetime.now().isoformat()
    }
    await manager.broadcast(message)


def get_connected_nodes() -> List[str]:
    """
    Get a list of connected node IDs.
    
    Returns:
        List[str]: List of node IDs
    """
    return manager.get_connected_nodes()


def is_node_connected(node_id: str) -> bool:
    """
    Check if a node is connected.
    
    Args:
        node_id: The ID of the node to check
    
    Returns:
        bool: True if the node is connected, False otherwise
    """
    return manager.is_connected(node_id)
