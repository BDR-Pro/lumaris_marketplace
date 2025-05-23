"""Pydantic schemas for node and job interactions in the Compute Marketplace API."""

from pydantic import BaseModel


class StatUpdate(BaseModel):
    """Schema for incoming stats from a compute node."""

    node_id: str
    cpu: float
    mem: float
    funds: float


class NodeOut(BaseModel):
    """Schema for returning node information."""

    id: int
    hostname: str
    status: str
    cpu_usage: float
    mem_usage: float

    model_config = {"from_attributes": True}


class JobIn(BaseModel):
    """Schema for submitting a new job."""

    node_id: int
    status: str
    duration: float
    cost: float


class JobOut(JobIn):
    """Schema for returning a submitted job."""

    id: int

    model_config = {"from_attributes": True}


class NodeAvailabilityUpdate(BaseModel):
    """Schema for updating node availability and capacity."""

    node_id: str
    cpu_available: float
    mem_available: int
    status: str


class JobAssignmentIn(BaseModel):
    """Schema for assigning a job to a node."""

    job_id: str
    node_id: str


class JobStatusUpdate(BaseModel):
    """Schema for reporting job execution status."""

    job_id: str
    status: str
    progress: float | None = None
    cpu_time_sec: int | None = None
    peak_memory_mb: int | None = None
