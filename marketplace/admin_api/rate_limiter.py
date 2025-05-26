"""
Rate limiting middleware for the Lumaris Marketplace API.

This module provides a rate limiter to protect API endpoints from abuse
by limiting the number of requests a client can make in a given time window.
"""

import time
from typing import Dict, List, Tuple, Optional
import logging
from fastapi import Request, Response
from fastapi.responses import JSONResponse
import os

# Configure logging
logger = logging.getLogger(__name__)

class RateLimiter:
    """
    A simple in-memory rate limiter that tracks requests by client IP.
    
    Attributes:
        rate_limit (int): Maximum number of requests allowed per time window
        time_window (int): Time window in seconds
        clients (Dict[str, List[float]]): Dictionary mapping client IDs to request timestamps
        whitelist (List[str]): List of client IDs that are exempt from rate limiting
    """
    
    def __init__(
        self, 
        rate_limit: int = 100, 
        time_window: int = 60,
        whitelist: Optional[List[str]] = None
    ):
        """
        Initialize the rate limiter.
        
        Args:
            rate_limit: Maximum number of requests allowed per time window
            time_window: Time window in seconds
            whitelist: List of client IDs that are exempt from rate limiting
        """
        self.rate_limit = rate_limit
        self.time_window = time_window
        self.clients: Dict[str, List[float]] = {}
        self.whitelist = whitelist or []
        
        logger.info(
            f"Rate limiter initialized with limit of {rate_limit} "
            f"requests per {time_window} seconds"
        )
    
    def is_rate_limited(self, client_id: str) -> Tuple[bool, int, int]:
        """
        Check if a client is rate limited.
        
        Args:
            client_id: Identifier for the client (typically IP address)
            
        Returns:
            Tuple containing:
            - Whether the client is rate limited (True if limited)
            - Current number of requests in the time window
            - Seconds until the oldest request expires
        """
        # Whitelist check
        if client_id in self.whitelist:
            return False, 0, 0
        
        current_time = time.time()
        
        # Initialize client if not exists
        if client_id not in self.clients:
            self.clients[client_id] = []
        
        # Remove timestamps outside the window
        valid_requests = []
        for timestamp in self.clients[client_id]:
            if current_time - timestamp < self.time_window:
                valid_requests.append(timestamp)
        
        self.clients[client_id] = valid_requests
        
        # Calculate time until reset
        time_until_reset = 0
        if valid_requests:
            oldest_request = min(valid_requests)
            time_until_reset = int(self.time_window - (current_time - oldest_request))
        
        # Check if rate limited
        is_limited = len(valid_requests) >= self.rate_limit
        
        # Add current request timestamp if not limited
        if not is_limited:
            self.clients[client_id].append(current_time)
        
        return is_limited, len(valid_requests), time_until_reset

    def get_headers(self, client_id: str) -> Dict[str, str]:
        """
        Get rate limit headers for a response.
        
        Args:
            client_id: Identifier for the client
            
        Returns:
            Dictionary of rate limit headers
        """
        is_limited, current_usage, time_until_reset = self.is_rate_limited(client_id)
        
        # Don't count this call in the usage
        if not is_limited and current_usage > 0:
            current_usage -= 1
        
        return {
            "X-RateLimit-Limit": str(self.rate_limit),
            "X-RateLimit-Remaining": str(max(0, self.rate_limit - current_usage)),
            "X-RateLimit-Reset": str(time_until_reset)
        }

async def rate_limit_middleware(request: Request, call_next, rate_limiter: RateLimiter):
    """
    FastAPI middleware for rate limiting.
    
    Args:
        request: The incoming request
        call_next: The next middleware or endpoint handler
        rate_limiter: The rate limiter instance
        
    Returns:
        The response from the next handler or a 429 Too Many Requests response
    """
    # Get client identifier (IP address)
    client_id = request.client.host if request.client else "unknown"
    
    # Check if client is rate limited
    is_limited, current_usage, time_until_reset = rate_limiter.is_rate_limited(client_id)
    
    # If rate limited, return 429 response
    if is_limited:
        logger.warning(f"Rate limit exceeded for client {client_id}")
        return JSONResponse(
            status_code=429,
            content={
                "error": "Too many requests",
                "detail": f"Rate limit of {rate_limiter.rate_limit} requests per {rate_limiter.time_window} seconds exceeded",
                "current_usage": current_usage,
                "retry_after": time_until_reset
            },
            headers=rate_limiter.get_headers(client_id)
        )
    
    # Process the request
    response = await call_next(request)
    
    # Add rate limit headers to response
    for header, value in rate_limiter.get_headers(client_id).items():
        response.headers[header] = value
    
    return response

def create_rate_limiter() -> RateLimiter:
    """
    Create a rate limiter instance with configuration from environment variables.
    
    Returns:
        Configured RateLimiter instance
    """
    rate_limit = int(os.getenv("RATE_LIMIT", "100"))
    time_window = int(os.getenv("RATE_LIMIT_WINDOW", "60"))
    whitelist_str = os.getenv("RATE_LIMIT_WHITELIST", "")
    whitelist = [ip.strip() for ip in whitelist_str.split(",")] if whitelist_str else []
    
    return RateLimiter(
        rate_limit=rate_limit,
        time_window=time_window,
        whitelist=whitelist
    )

