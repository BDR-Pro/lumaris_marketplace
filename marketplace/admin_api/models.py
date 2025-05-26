# pylint: disable=too-few-public-methods
"""SQLAlchemy models for the Admin API database schema."""

from datetime import datetime

from sqlalchemy import Column, DateTime, Float
from sqlalchemy import ForeignKey as _ForeignKey
from sqlalchemy import Integer, String
from sqlalchemy.orm import declarative_base
from sqlalchemy.orm import relationship as _relationship

Base = declarative_base()


class Node(Base):
    """Represents a computing node in the system."""

    __tablename__ = "nodes"

    id = Column(Integer, primary_key=True, index=True)
    hostname = Column(String)
    status = Column(String)
    cpu_usage = Column(Float)
    mem_usage = Column(Float)


class NodeToken(Base):
    """Represents a token for a compute node."""

    __tablename__ = "node_tokens"
    id = Column(Integer, primary_key=True)
    token = Column(String, unique=True)
    node_id = Column(String)
    expires_at = Column(DateTime)


class Job(Base):
    """Represents a compute job submitted to the system."""

    __tablename__ = "jobs"

    id = Column(Integer, primary_key=True, index=True)
    node_id = Column(Integer)
    status = Column(String)
    duration = Column(Float)
    cost = Column(Float)


class JobAssignment(Base):
    """Tracks the assignment and execution state of a job."""

    __tablename__ = "job_assignments"

    id = Column(Integer, primary_key=True, index=True)
    job_id = Column(String)
    node_id = Column(String)
    assigned_at = Column(DateTime, default=datetime.utcnow)
    started_at = Column(DateTime, nullable=True)
    completed_at = Column(DateTime, nullable=True)
    status = Column(String)


class NodeCapability(Base):
    """Represents the available resources of a node."""

    __tablename__ = "node_capabilities"

    id = Column(Integer, primary_key=True)
    node_id = Column(String, unique=True)
    cpu_cores = Column(Float)
    memory_mb = Column(Integer)
    reliability_score = Column(Float, default=1.0)
    last_seen_at = Column(DateTime, default=datetime.utcnow)


class NodeReputation(Base):
    """Tracks reputation metrics for compute nodes."""
    
    __tablename__ = "node_reputations"
    
    id = Column(Integer, primary_key=True)
    node_id = Column(String, unique=True)
    reputation_score = Column(Float, default=0.5)  # 0.0 to 1.0
    successful_jobs = Column(Integer, default=0)
    failed_jobs = Column(Integer, default=0)
    total_uptime_hours = Column(Float, default=0.0)
    avg_response_time_ms = Column(Float, default=0.0)
    avg_job_completion_time_sec = Column(Float, default=0.0)
    first_seen = Column(DateTime, nullable=True)
    last_updated = Column(DateTime, default=datetime.utcnow)
