from fastapi import WebSocket, WebSocketDisconnect, Depends, HTTPException, status
from typing import Dict, List, Optional, Any
import json
import logging
import asyncio
from datetime import datetime, timedelta

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
        # Connection pool status
        self.pool_status = {
            "total_connections_ever": 0,
            "max_concurrent_connections": 0,
            "connection_errors": 0,
            "heartbeat_failures": 0
        }
        # Heartbeat monitor task
        self.heartbeat_monitor_task = None
        # Heartbeat timeout in seconds
        self.heartbeat_timeout = 120  # 2 minutes
        # Heartbeat check interval in seconds
        self.heartbeat_check_interval = 30  # 30 seconds
    
    async def connect(self, websocket: WebSocket, node_id: str):
        await websocket.accept()
        self.active_connections[node_id] = websocket
        self.last_heartbeat[node_id] = datetime.now()
        
        # Update connection stats
        self.pool_status["total_connections_ever"] += 1
        current_connections = len(self.active_connections)
        if current_connections > self.pool_status["max_concurrent_connections"]:
            self.pool_status["max_concurrent_connections"] = current_connections
        
        logger.info(f"Node {node_id} connected via WebSocket. Active connections: {current_connections}")
        increment_websocket_connections()
    
    def disconnect(self, node_id: str, reason: str = "normal"):
        if node_id in self.active_connections:
            del self.active_connections[node_id]
        if node_id in self.last_heartbeat:
            del self.last_heartbeat[node_id]
        
        if reason != "normal":
            logger.warning(f"Node {node_id} disconnected from WebSocket: {reason}")
            if reason == "heartbeat_timeout":
                self.pool_status["heartbeat_failures"] += 1
            elif reason == "error":
                self.pool_status["connection_errors"] += 1
        else:
            logger.info(f"Node {node_id} disconnected from WebSocket")
        
        decrement_websocket_connections()
    
    async def send_message(self, node_id: str, message: Dict[str, Any]):
        if node_id in self.active_connections:
            try:
                websocket = self.active_connections[node_id]
                await websocket.send_json(message)
                logger.info(f"Message sent to node {node_id}: {message}")
                return True
            except Exception as e:
                logger.error(f"Error sending message to node {node_id}: {str(e)}")
                self.disconnect(node_id, reason="error")
                return False
        logger.warning(f"Attempted to send message to disconnected node {node_id}")
        return False
    
    async def broadcast(self, message: Dict[str, Any]):
        disconnected_nodes = []
        for node_id, websocket in self.active_connections.items():
            try:
                await websocket.send_json(message)
            except Exception as e:
                logger.error(f"Error broadcasting to node {node_id}: {str(e)}")
                disconnected_nodes.append(node_id)
        
        # Clean up disconnected nodes
        for node_id in disconnected_nodes:
            self.disconnect(node_id, reason="error")
        
        logger.info(f"Broadcast message sent to {len(self.active_connections)} nodes: {message}")
    
    def update_heartbeat(self, node_id: str):
        if node_id in self.active_connections:
            self.last_heartbeat[node_id] = datetime.now()
            logger.debug(f"Heartbeat updated for node {node_id}")
    
    def get_connected_nodes(self) -> List[str]:
        return list(self.active_connections.keys())
    
    def is_connected(self, node_id: str) -> bool:
        return node_id in self.active_connections
    
    def get_connection_stats(self) -> Dict[str, Any]:
        stats = self.pool_status.copy()
        stats["current_connections"] = len(self.active_connections)
        return stats
    
    async def monitor_heartbeats(self):
        """Monitor node heartbeats and disconnect inactive nodes."""
        logger.info("Starting WebSocket heartbeat monitor")
        while True:
            try:
                current_time = datetime.now()
                inactive_nodes = []
                
                for node_id, last_time in self.last_heartbeat.items():
                    # If no heartbeat for the timeout period, consider node inactive
                    if current_time - last_time > timedelta(seconds=self.heartbeat_timeout):
                        inactive_nodes.append(node_id)
                
                # Disconnect inactive nodes
                for node_id in inactive_nodes:
                    if node_id in self.active_connections:
                        try:
                            await self.active_connections[node_id].close(code=1000)
                        except Exception:
                            pass  # Already disconnected
                        self.disconnect(node_id, reason="heartbeat_timeout")
                        logger.warning(f"Node {node_id} disconnected due to heartbeat timeout")
                
                # Check every interval
                await asyncio.sleep(self.heartbeat_check_interval)
            except Exception as e:
                logger.error(f"Error in heartbeat monitor: {str(e)}")
                await asyncio.sleep(self.heartbeat_check_interval)
    
    def start_heartbeat_monitor(self):
        """Start the heartbeat monitor task."""
        if self.heartbeat_monitor_task is None or self.heartbeat_monitor_task.done():
            self.heartbeat_monitor_task = asyncio.create_task(self.monitor_heartbeats())
            logger.info("Heartbeat monitor started")
    
    def stop_heartbeat_monitor(self):
        """Stop the heartbeat monitor task."""
        if self.heartbeat_monitor_task and not self.heartbeat_monitor_task.done():
            self.heartbeat_monitor_task.cancel()
            logger.info("Heartbeat monitor stopped")


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
            "timestamp": datetime.now().isoformat(),
            "heartbeat_interval": 60,  # Recommend heartbeat every 60 seconds
            "heartbeat_timeout": manager.heartbeat_timeout
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
        manager.disconnect(node.id, reason="error")


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


def get_connection_stats() -> Dict[str, Any]:
    """
    Get statistics about WebSocket connections.
    
    Returns:
        Dict[str, Any]: Connection statistics
    """
    return manager.get_connection_stats()
