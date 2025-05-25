"""Main entrypoint for the Admin API of the Compute Marketplace."""

import logging
from rich.console import Console
from rich.logging import RichHandler
from rich.panel import Panel
from rich import print as rprint

from admin_api.auth import router as auth_router

# These are imported in other files for DB initialization
# Keeping them for side effects (e.g., Alembic, if used)
from admin_api.database import engine as _engine
from admin_api.jobs import router as job_router
from admin_api.matchmaking import router as matchmaking_router
from admin_api.models import Base as _Base
from admin_api.nodes import router as node_router
from fastapi import FastAPI

# Configure Rich console and logging
console = Console()
logging.basicConfig(
    level=logging.INFO,
    format="%(message)s",
    datefmt="[%X]",
    handlers=[RichHandler(rich_tracebacks=True)]
)
log = logging.getLogger("admin_api")

def create_app() -> FastAPI:
    """Create and configure the FastAPI application instance."""
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
    
    app = FastAPI(title="Admin API for Compute Marketplace")
    app.include_router(auth_router, prefix="/auth")
    app.include_router(node_router, prefix="/nodes")
    app.include_router(job_router, prefix="/jobs")
    app.include_router(matchmaking_router, prefix="/matchmaking")
    
    log.info("[bold green]✓[/bold green] Admin API initialized successfully")
    
    @app.get("/health")
    async def health_check():
        """Health check endpoint."""
        return {"status": "healthy"}
    
    return app


# Used by production (e.g. uvicorn main:app)
app = create_app()

