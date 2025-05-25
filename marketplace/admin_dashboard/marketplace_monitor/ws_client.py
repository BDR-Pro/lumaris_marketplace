"""
WebSocket client for real-time updates from the marketplace.
"""
import json
import logging
import asyncio
import websockets
from django.conf import settings
from datetime import datetime
from typing import Dict, List, Any, Optional, Callable, Set
from uuid import uuid4

logger = logging.getLogger(__name__)

class WebSocketClient:
    """Client for WebSocket communication with the marketplace."""
    
    def __init__(self):
        self.ws_url = settings.WS_URL
        self.connection = None
        self.connected = False
        self.handlers = set()
        self.reconnect_delay = 1.0  # Initial reconnect delay in seconds
        self.max_reconnect_delay = 60.0  # Maximum reconnect delay in seconds
        self.heartbeat_interval = 30.0  # Heartbeat interval in seconds
        self.heartbeat_task = None
        self.receive_task = None
    
    async def connect(self):
        """Connect to the WebSocket server."""
        if self.connected:
            return
        
        try:
            self.connection = await websockets.connect(self.ws_url)
            self.connected = True
            logger.info(f"Connected to WebSocket server at {self.ws_url}")
            
            # Start heartbeat and receive tasks
            self.heartbeat_task = asyncio.create_task(self._heartbeat_loop())
            self.receive_task = asyncio.create_task(self._receive_loop())
            
            # Reset reconnect delay on successful connection
            self.reconnect_delay = 1.0
            
            # Send identification message
            await self.send_message("identify", {
                "client_type": "admin_dashboard",
                "client_id": str(uuid4()),
                "client_version": "1.0.0"
            })
        except Exception as e:
            logger.error(f"Failed to connect to WebSocket server: {str(e)}")
            self.connected = False
            
            # Schedule reconnection
            await self._schedule_reconnect()
    
    async def disconnect(self):
        """Disconnect from the WebSocket server."""
        if not self.connected:
            return
        
        try:
            # Cancel tasks
            if self.heartbeat_task:
                self.heartbeat_task.cancel()
            if self.receive_task:
                self.receive_task.cancel()
            
            # Close connection
            if self.connection:
                await self.connection.close()
            
            logger.info("Disconnected from WebSocket server")
        except Exception as e:
            logger.error(f"Error during WebSocket disconnection: {str(e)}")
        finally:
            self.connected = False
            self.connection = None
    
    async def send_message(self, message_type: str, data: Dict):
        """Send a message to the WebSocket server."""
        if not self.connected:
            logger.warning("Cannot send message: not connected to WebSocket server")
            return False
        
        try:
            message = {
                "type": message_type,
                "request_id": str(uuid4()),
                "role": "admin",
                "data": data,
                "timestamp": datetime.utcnow().isoformat()
            }
            
            await self.connection.send(json.dumps(message))
            return True
        except Exception as e:
            logger.error(f"Failed to send WebSocket message: {str(e)}")
            self.connected = False
            await self._schedule_reconnect()
            return False
    
    def register_handler(self, handler: Callable[[Dict], None]):
        """Register a message handler."""
        self.handlers.add(handler)
        return handler
    
    def unregister_handler(self, handler: Callable[[Dict], None]):
        """Unregister a message handler."""
        if handler in self.handlers:
            self.handlers.remove(handler)
    
    async def _receive_loop(self):
        """Loop to receive messages from the WebSocket server."""
        if not self.connected:
            return
        
        try:
            async for message in self.connection:
                try:
                    data = json.loads(message)
                    # Process message with all registered handlers
                    for handler in self.handlers:
                        try:
                            handler(data)
                        except Exception as e:
                            logger.error(f"Error in WebSocket message handler: {str(e)}")
                except json.JSONDecodeError:
                    logger.warning(f"Received invalid JSON: {message}")
        except websockets.exceptions.ConnectionClosed:
            logger.warning("WebSocket connection closed")
            self.connected = False
            await self._schedule_reconnect()
        except Exception as e:
            logger.error(f"Error in WebSocket receive loop: {str(e)}")
            self.connected = False
            await self._schedule_reconnect()
    
    async def _heartbeat_loop(self):
        """Loop to send heartbeat messages to the WebSocket server."""
        while self.connected:
            try:
                await asyncio.sleep(self.heartbeat_interval)
                if self.connected:
                    await self.send_message("heartbeat", {
                        "timestamp": datetime.utcnow().isoformat()
                    })
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Error in WebSocket heartbeat loop: {str(e)}")
                self.connected = False
                await self._schedule_reconnect()
                break
    
    async def _schedule_reconnect(self):
        """Schedule a reconnection attempt with exponential backoff."""
        await asyncio.sleep(self.reconnect_delay)
        
        # Increase reconnect delay with exponential backoff
        self.reconnect_delay = min(self.reconnect_delay * 2, self.max_reconnect_delay)
        
        # Attempt to reconnect
        await self.connect()


# Create a singleton instance
ws_client = WebSocketClient()

