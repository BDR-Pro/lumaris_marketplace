from fastapi import FastAPI, Depends, HTTPException, status, WebSocket, Query
from fastapi.middleware.cors import CORSMiddleware
from sqlalchemy.orm import Session
import os
from rich.console import Console

from . import models, schemas, nodes, jobs, matchmaking, auth
from .database import engine, get_db
from .websocket import handle_websocket

# Configure Rich console and logging
console = Console()

# Create database tables
models.Base.metadata.create_all(bind=engine)

# Create FastAPI app
app = FastAPI(
    title="Lumaris Marketplace API",
    description="API for the Lumaris decentralized marketplace",
    version="0.1.0"
)

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
    return {"message": "Welcome to the Lumaris Marketplace API"}

# Health check endpoint
@app.get("/health")
def health_check():
    return {"status": "healthy"}

# WebSocket endpoint
@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket, token: str = Query(...)):
    await handle_websocket(websocket, token)

