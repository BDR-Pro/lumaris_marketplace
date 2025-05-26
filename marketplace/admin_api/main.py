"""Main entrypoint for the Admin API of the Compute Marketplace."""

import logging
import os
from rich.console import Console
from rich.logging import RichHandler
from rich.panel import Panel
from fastapi import FastAPI, WebSocket, Query
from fastapi.middleware.cors import CORSMiddleware
from sqlalchemy.orm import Session

from . import models, schemas, nodes, jobs, matchmaking, auth
from .database import engine, get_db
from .websocket import handle_websocket

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

# WebSocket endpoint
@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket, token: str = Query(...)):
    log.info("WebSocket connection initiated")
    await handle_websocket(websocket, token)
