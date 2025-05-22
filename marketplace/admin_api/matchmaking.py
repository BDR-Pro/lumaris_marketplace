# admin_api/matchmaking.py
from datetime import datetime

from fastapi import APIRouter, Depends, Header, HTTPException
from sqlalchemy.orm import Session

from admin_api.database import SessionLocal
from admin_api.models import JobAssignment, NodeCapability
from admin_api.schemas import (JobAssignmentIn, JobStatusUpdate,
                               NodeAvailabilityUpdate)

router = APIRouter()


def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()


API_KEY = "super-secret"  # Replace with env var in production


def validate_key(x_api_key: str = Header(...)):
    if x_api_key != API_KEY:
        raise HTTPException(status_code=403, detail="Invalid API key")


@router.post("/node/update_availability", dependencies=[Depends(validate_key)])
def update_availability(update: NodeAvailabilityUpdate, db: Session = Depends(get_db)):
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
def assign_job(assign: JobAssignmentIn, db: Session = Depends(get_db)):
    job = JobAssignment(job_id=assign.job_id, node_id=assign.node_id, status="assigned")
    db.add(job)
    db.commit()
    return {"assigned": True}


@router.post("/job/status", dependencies=[Depends(validate_key)])
def job_status(update: JobStatusUpdate, db: Session = Depends(get_db)):
    job = db.query(JobAssignment).filter_by(job_id=update.job_id).first()
    if not job:
        raise HTTPException(status_code=404, detail="Job not found")
    job.status = update.status
    if update.status == "completed":
        job.completed_at = datetime.utcnow()
    db.commit()
    return {"status_updated": True}
