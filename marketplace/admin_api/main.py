"""Main entrypoint for the Admin API of the Compute Marketplace."""

from admin_api.auth import router as auth_router

# These are imported in other files for DB initialization
# Keeping them for side effects (e.g., Alembic, if used)
from admin_api.database import engine as _engine
from admin_api.jobs import router as job_router
from admin_api.matchmaking import router as matchmaking_router
from admin_api.models import Base as _Base
from admin_api.nodes import router as node_router
from fastapi import FastAPI


def create_app() -> FastAPI:
    """Create and configure the FastAPI application instance."""
    app = FastAPI(title="Admin API for Compute Marketplace")
    app.include_router(auth_router, prefix="/auth")
    app.include_router(node_router, prefix="/nodes")
    app.include_router(job_router, prefix="/jobs")
    app.include_router(matchmaking_router, prefix="/matchmaking")
    return app


# Used by production (e.g. uvicorn main:app)
app = create_app()
