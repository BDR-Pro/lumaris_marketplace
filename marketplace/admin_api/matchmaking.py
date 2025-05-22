"""Matchmaking routes for assigning and tracking compute jobs."""

from datetime import datetime

from fastapi import APIRouter, Depends, Header, HTTPException
from sqlalchemy.orm import Session

from admin_api.database import get_db
from admin_api.models import JobAssignment, NodeCapability
from admin_api.schemas import (
    JobAssignmentIn,
    JobStatusUpdate,
    NodeAvailabilityUpdate,
)

router = APIRouter()

API_KEY = "super-secret"  # TODO: Replace with environment variable in production


def validate_key(x_api_key: str = Header(...)) -> None:
    """Validate the provided API key from headers."""
    if x_api_key != API_KEY:
        raise HTTPException(status_code=403, detail="Invalid API key")


@router.post("/node/update_availability", dependencies=[Depends(validate_key)])
def update_availability(
    update: NodeAvailabilityUpdate, db: Session = Depends(get_db)
) -> dict:
    """Update available CPU and memory for a given node."""
    node = db.query(NodeCapability).filter_by(node_id=update.node_id).first()
    if not node:
        node = NodeCapability(node_id=update.node_id)
        db.add(node)
    node.cpu_cores = update.cpu_available
    node.memory_mb = update.mem_available
    node.last_seen_at = datetime.utcnow()
    db.commit()
    return {"success": True}


@router.post("/job/assign", dependencies=[Depends(validate_key)])
def assign_job(assign: JobAssignmentIn, db: Session = Depends(get_db)) -> dict:
    """Assign a job to a node."""
    job = JobAssignment(job_id=assign.job_id, node_id=assign.node_id, status="assigned")
    db.add(job)
    db.commit()
    return {"assigned": True}


@router.post("/job/status", dependencies=[Depends(validate_key)])
def job_status(update: JobStatusUpdate, db: Session = Depends(get_db)) -> dict:
    """Update the status of an assigned job."""
    job = db.query(JobAssignment).filter_by(job_id=update.job_id).first()
    if not job:
        raise HTTPException(status_code=404, detail="Job not found")
    job.status = update.status
    if update.status == "completed":
        job.completed_at = datetime.utcnow()
    db.commit()
    return {"status_updated": True}
