
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

    model_config = {
        "from_attributes": True
    }
    
    
class JobIn(BaseModel):
    node_id: int
    status: str
    duration: float
    cost: float

class JobOut(JobIn):
    id: int

    model_config = {
        "from_attributes": True
    }
