from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from admin_api.database import SessionLocal
from admin_api.models import Job
from admin_api.schemas import JobOut, JobIn
from typing import List

router = APIRouter()

def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()

@router.get("/", response_model=List[JobOut])
def list_jobs(db: Session = Depends(get_db)):
    jobs = db.query(Job).all()
    return [JobOut.model_validate(j) for j in jobs]

@router.post("/", response_model=JobOut)
def submit_job(job_in: JobIn, db: Session = Depends(get_db)):
    job = Job(**job_in.dict())
    db.add(job)
    db.commit()
    db.refresh(job)
    return JobOut.model_validate(job)
