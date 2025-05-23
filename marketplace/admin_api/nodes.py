"""FastAPI router for handling compute node registration and status updates."""

import logging
import uuid
from datetime import datetime, timedelta
from typing import List

from admin_api.database import get_db
from admin_api.models import Node, NodeToken
from admin_api.schemas import NodeOut, StatUpdate
from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

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


@router.post("/token/{node_id}")
def issue_token(node_id: str, db: Session = Depends(get_db)):
    """issue a token for a compute node."""
    token = str(uuid.uuid4())
    expires = datetime.now() + timedelta(days=7)
    db_token = NodeToken(token=token, node_id=node_id, expires_at=expires)
    db.add(db_token)
    db.commit()
    return {"token": token, "expires_at": expires.isoformat()}
