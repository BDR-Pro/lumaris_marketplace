from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from admin_api.database import SessionLocal
from admin_api.models import Node
from admin_api.schemas import NodeOut, StatUpdate
from typing import List

router = APIRouter()

# Dependency
def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()

@router.post("/update", response_model=NodeOut)
def update_node_stats(stat: StatUpdate, db: Session = Depends(get_db)):
    print(f"Stat from {stat.node_id}: CPU={stat.cpu}, MEM={stat.mem}, FUNDS={stat.funds}")

    # Check if node already exists
    node = db.query(Node).filter(Node.hostname == stat.node_id).first()
    if not node:
        node = Node(
            hostname=stat.node_id,
            status="online",
            cpu_usage=stat.cpu,
            mem_usage=stat.mem
        )
        db.add(node)
    else:
        node.cpu_usage = stat.cpu
        node.mem_usage = stat.mem
        node.status = "online"

    db.commit()
    db.refresh(node)
    return NodeOut.model_validate(node)

@router.get("/", response_model=List[NodeOut])
def list_nodes(db: Session = Depends(get_db)):
    nodes = db.query(Node).all()
    return [NodeOut.model_validate(n) for n in nodes]

@router.post("/", response_model=NodeOut)
def register_node(stat: StatUpdate, db: Session = Depends(get_db)):
    node = Node(
        hostname=stat.node_id,
        status="online",
        cpu_usage=stat.cpu,
        mem_usage=stat.mem
    )
    db.add(node)
    db.commit()
    db.refresh(node)
    return NodeOut.model_validate(node)
