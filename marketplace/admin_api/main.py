"""Main entrypoint for the Admin API of the Compute Marketplace."""

import logging
import os
import asyncio
from rich.console import Console
from rich.logging import RichHandler
from rich.panel import Panel
from fastapi import FastAPI, WebSocket, Query, Request
from fastapi.middleware.cors import CORSMiddleware
from sqlalchemy.orm import Session
from functools import partial

from . import models, schemas, nodes, jobs, matchmaking, auth
from .database import engine, get_db
from .websocket import handle_websocket, manager as websocket_manager
from .rate_limiter import create_rate_limiter, rate_limit_middleware
from .metrics import setup_metrics

# Configure Rich console and logging
console = Console()
logging.basicConfig(
    level=logging.INFO,
    format="%(message)s",
    datefmt="[%X]",
    handlers=[RichHandler(rich_tracebacks=True)]
)
log = logging.getLogger("admin_api")

# Create database tables
models.Base.metadata.create_all(bind=engine)

# Create rate limiter
rate_limiter = create_rate_limiter()

# Create FastAPI app
app = FastAPI(
    title="Lumaris Marketplace API",
    description="API for the Lumaris decentralized marketplace",
    version="0.1.0"
)

# Print a beautiful banner
console.print(Panel.fit(
    """
    [bold blue]██╗     ██╗   ██╗███╗   ███╗ █████╗ ██████╗ ██╗███████╗[/bold blue]
    [bold blue]██║     ██║   ██║████╗ ████║██╔══██╗██╔══██╗██║██╔════╝[/bold blue]
    [bold blue]██║     ██║   ██║██╔████╔██║███████║██████╔╝██║███████╗[/bold blue]
    [bold blue]██║     ██║   ██║██║╚██╔╝██║██╔══██║██╔══██╗██║╚════██║[/bold blue]
    [bold blue]███████╗╚██████╔╝██║ ╚═╝ ██║██║  ██║██║  ██║██║███████║[/bold blue]
    [bold blue]╚══════╝ ╚═════╝ ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚══════╝[/bold blue]
    
    [bold green]Admin API for Compute Marketplace[/bold green]
    """,
    title="[bold yellow]Lumaris Marketplace[/bold yellow]",
    border_style="green",
))

# Configure CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=os.getenv("CORS_ALLOWED_ORIGINS", "http://localhost:8000").split(","),
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Add rate limiting middleware
@app.middleware("http")
async def rate_limiting(request: Request, call_next):
    return await rate_limit_middleware(request, call_next, rate_limiter)

# Setup Prometheus metrics
setup_metrics(app)

# Include routers
app.include_router(nodes.router, prefix="/nodes", tags=["nodes"])
app.include_router(jobs.router, prefix="/jobs", tags=["jobs"])
app.include_router(matchmaking.router, prefix="/matchmaking", tags=["matchmaking"])

# Root endpoint
@app.get("/")
def read_root():
    log.info("Root endpoint accessed")
    return {"message": "Welcome to the Lumaris Marketplace API"}

# Health check endpoint
@app.get("/health")
def health_check():
    log.info("Health check endpoint accessed")
    return {"status": "healthy"}

# WebSocket connection stats endpoint
@app.get("/ws/stats")
def websocket_stats():
    """Get statistics about WebSocket connections."""
    log.info("WebSocket stats endpoint accessed")
    return websocket_manager.get_connection_stats()

# WebSocket endpoint
@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket, token: str = Query(...)):
    log.info("WebSocket connection initiated")
    await handle_websocket(websocket, token)

# Startup event
@app.on_event("startup")
async def startup_event():
    """Initialize services on startup."""
    log.info("Starting up the API server")
    # Start the WebSocket heartbeat monitor
    websocket_manager.start_heartbeat_monitor()

# Shutdown event
@app.on_event("shutdown")
async def shutdown_event():
    """Clean up resources on shutdown."""
    log.info("Shutting down the API server")
    # Stop the WebSocket heartbeat monitor
    websocket_manager.stop_heartbeat_monitor()
