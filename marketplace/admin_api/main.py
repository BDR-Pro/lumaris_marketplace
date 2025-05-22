# admin_api/main.py
from fastapi import FastAPI
from admin_api.auth import router as auth_router
from admin_api.models import Base
from admin_api.database import engine

from admin_api.nodes import router as node_router
from admin_api.jobs import router as job_router
from admin_api.matchmaking import router as matchmaking_router

app = FastAPI(title="Admin API for Compute Marketplace")

Base.metadata.create_all(bind=engine)

app.include_router(auth_router, prefix="/auth")


app.include_router(node_router, prefix="/nodes")
app.include_router(job_router, prefix="/jobs")
app.include_router(matchmaking_router, prefix="/matchmaking")
