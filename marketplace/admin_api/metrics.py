"""
Metrics collection and monitoring for the Lumaris Marketplace API.

This module provides integration with Prometheus for collecting and exposing
metrics about the API's performance and usage.
"""

import time
from typing import Callable, Dict
from fastapi import FastAPI, Request, Response
import logging
import os
from prometheus_client import Counter, Histogram, Gauge, Info, generate_latest, CONTENT_TYPE_LATEST

# Configure logging
logger = logging.getLogger(__name__)

# Define metrics
REQUEST_COUNT = Counter(
    "lumaris_http_requests_total",
    "Total number of HTTP requests",
    ["method", "endpoint", "status_code"]
)

REQUEST_LATENCY = Histogram(
    "lumaris_http_request_duration_seconds",
    "HTTP request latency in seconds",
    ["method", "endpoint"]
)

ACTIVE_REQUESTS = Gauge(
    "lumaris_http_requests_active",
    "Number of active HTTP requests"
)

WEBSOCKET_CONNECTIONS = Gauge(
    "lumaris_websocket_connections_active",
    "Number of active WebSocket connections"
)

NODE_COUNT = Gauge(
    "lumaris_nodes_total",
    "Total number of registered nodes",
    ["status"]
)

JOB_COUNT = Gauge(
    "lumaris_jobs_total",
    "Total number of jobs",
    ["status"]
)

API_INFO = Info(
    "lumaris_api",
    "Information about the Lumaris API"
)

async def metrics_middleware(request: Request, call_next: Callable) -> Response:
    """
    Middleware to collect metrics for each request.
    
    Args:
        request: The incoming request
        call_next: The next middleware or endpoint handler
        
    Returns:
        The response from the next handler
    """
    # Skip metrics collection for the metrics endpoint itself
    if request.url.path == "/metrics":
        return await call_next(request)
    
    # Extract method and path for labels
    method = request.method
    endpoint = request.url.path
    
    # Track active requests
    ACTIVE_REQUESTS.inc()
    
    # Track request latency
    start_time = time.time()
    
    # Process the request
    try:
        response = await call_next(request)
        status_code = response.status_code
    except Exception as e:
        # If an exception occurs, count it as a 500 error
        logger.exception("Exception during request processing")
        status_code = 500
        raise
    finally:
        # Record request duration
        duration = time.time() - start_time
        REQUEST_LATENCY.labels(method=method, endpoint=endpoint).observe(duration)
        
        # Record request count
        REQUEST_COUNT.labels(
            method=method, 
            endpoint=endpoint, 
            status_code=status_code
        ).inc()
        
        # Decrement active requests
        ACTIVE_REQUESTS.dec()
    
    return response

async def metrics_endpoint():
    """
    Endpoint to expose Prometheus metrics.
    
    Returns:
        Raw Prometheus metrics in text format
    """
    return Response(
        content=generate_latest(),
        media_type=CONTENT_TYPE_LATEST
    )

def setup_metrics(app: FastAPI):
    """
    Set up metrics collection and exposure for a FastAPI app.
    
    Args:
        app: The FastAPI application
    """
    # Set API info
    API_INFO.info({
        "version": app.version,
        "title": app.title,
        "environment": os.getenv("ENVIRONMENT", "development")
    })
    
    # Add metrics middleware
    app.middleware("http")(metrics_middleware)
    
    # Add metrics endpoint
    app.add_route("/metrics", metrics_endpoint)
    
    logger.info("Prometheus metrics configured and exposed at /metrics")

def increment_websocket_connections():
    """Increment the count of active WebSocket connections."""
    WEBSOCKET_CONNECTIONS.inc()

def decrement_websocket_connections():
    """Decrement the count of active WebSocket connections."""
    WEBSOCKET_CONNECTIONS.dec()

def update_node_counts(status_counts: Dict[str, int]):
    """
    Update the node count metrics.
    
    Args:
        status_counts: Dictionary mapping node status to count
    """
    for status, count in status_counts.items():
        NODE_COUNT.labels(status=status).set(count)

def update_job_counts(status_counts: Dict[str, int]):
    """
    Update the job count metrics.
    
    Args:
        status_counts: Dictionary mapping job status to count
    """
    for status, count in status_counts.items():
        JOB_COUNT.labels(status=status).set(count)

