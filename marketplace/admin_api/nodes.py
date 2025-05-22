"""FastAPI router for handling compute node registration and status updates."""

from typing import List
import logging

from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from admin_api.models import Node
from admin_api.schemas import NodeOut, StatUpdate
from admin_api.database import get_db

router = APIRouter()
logger = logging.getLogger(__name__)


@router.post("/update", response_model=NodeOut)
def update_node_stats(stat: StatUpdate, db: Session = Depends(get_db)):
    """
    Update the CPU and memory usage stats of an existing node.
    Creates the node if it does not exist.
    """
    logger.info(
        "Stat from %s: CPU=%.2f, MEM=%.2f, FUNDS=%.2f",
        stat.node_id,
        stat.cpu,
        stat.mem,
        stat.funds,
    )

    node = db.query(Node).filter(Node.hostname == stat.node_id).first()
    if not node:
        node = Node(
            hostname=stat.node_id,
            status="online",
            cpu_usage=stat.cpu,
            mem_usage=stat.mem,
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
    """List all registered compute nodes."""
    nodes = db.query(Node).all()
    return [NodeOut.model_validate(n) for n in nodes]


@router.post("/", response_model=NodeOut)
def register_node(stat: StatUpdate, db: Session = Depends(get_db)):
    """Register a new compute node with initial stats."""
    node = Node(
        hostname=stat.node_id,
        status="online",
        cpu_usage=stat.cpu,
        mem_usage=stat.mem,
    )
    db.add(node)
    db.commit()
    db.refresh(node)
    return NodeOut.model_validate(node)
