from pydantic import BaseModel


class StatUpdate(BaseModel):
    node_id: str
    cpu: float
    mem: float
    funds: float


class NodeOut(BaseModel):
    id: int
    hostname: str
    status: str
    cpu_usage: float
    mem_usage: float

    model_config = {"from_attributes": True}


class JobIn(BaseModel):
    node_id: int
    status: str
    duration: float
    cost: float


class JobOut(JobIn):
    id: int

    model_config = {"from_attributes": True}


class NodeAvailabilityUpdate(BaseModel):
    node_id: str
    cpu_available: float
    mem_available: int
    status: str


class JobAssignmentIn(BaseModel):
    job_id: str
    node_id: str


class JobStatusUpdate(BaseModel):
    job_id: str
    status: str
    progress: float | None = None
    cpu_time_sec: int | None = None
    peak_memory_mb: int | None = None
