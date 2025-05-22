from datetime import datetime

from sqlalchemy import Column, DateTime, Float, Integer, String
from sqlalchemy.orm import declarative_base

Base = declarative_base()


class Node(Base):
    __tablename__ = "nodes"
    id = Column(Integer, primary_key=True, index=True)
    hostname = Column(String)
    status = Column(String)
    cpu_usage = Column(Float)
    mem_usage = Column(Float)


class Job(Base):
    __tablename__ = "jobs"
    id = Column(Integer, primary_key=True, index=True)
    node_id = Column(Integer)
    status = Column(String)
    duration = Column(Float)
    cost = Column(Float)


class JobAssignment(Base):
    __tablename__ = "job_assignments"
    id = Column(Integer, primary_key=True, index=True)
    job_id = Column(String)
    node_id = Column(String)
    assigned_at = Column(DateTime, default=datetime.utcnow)
    started_at = Column(DateTime, nullable=True)
    completed_at = Column(DateTime, nullable=True)
    status = Column(String)


class NodeCapability(Base):
    __tablename__ = "node_capabilities"
    id = Column(Integer, primary_key=True)
    node_id = Column(String, unique=True)
    cpu_cores = Column(Float)
    memory_mb = Column(Integer)
    reliability_score = Column(Float, default=1.0)
    last_seen_at = Column(DateTime, default=datetime.utcnow)
