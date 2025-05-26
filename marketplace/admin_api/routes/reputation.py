"""
API routes for the node reputation system.
"""

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session
from typing import List, Dict, Any, Optional

from ..database import get_db
from ..reputation import ReputationManager

router = APIRouter()

@router.get("/")
def get_top_nodes(
    limit: int = Query(10, ge=1, le=100),
    db: Session = Depends(get_db)
):
    """
    Get the top nodes by reputation score.
    
    Args:
        limit: Maximum number of nodes to return (1-100)
        db: Database session
        
    Returns:
        List of node reputation data
    """
    reputation_manager = ReputationManager(db)
    return reputation_manager.get_top_nodes(limit=limit)

@router.get("/{node_id}")
def get_node_reputation(
    node_id: str,
    db: Session = Depends(get_db)
):
    """
    Get the reputation data for a specific node.
    
    Args:
        node_id: The ID of the node
        db: Database session
        
    Returns:
        Node reputation data
    """
    reputation_manager = ReputationManager(db)
    return reputation_manager.get_node_reputation(node_id)

@router.post("/{node_id}/job")
def update_reputation_after_job(
    node_id: str,
    job_id: str,
    success: bool,
    response_time_ms: Optional[float] = None,
    completion_time_sec: Optional[float] = None,
    db: Session = Depends(get_db)
):
    """
    Update node reputation after job completion.
    
    Args:
        node_id: The ID of the node
        job_id: The ID of the job
        success: Whether the job was successful
        response_time_ms: Response time in milliseconds
        completion_time_sec: Job completion time in seconds
        db: Database session
        
    Returns:
        Updated reputation data
    """
    reputation_manager = ReputationManager(db)
    
    # Update reputation
    reputation_manager.update_reputation_after_job(
        node_id=node_id,
        job_id=job_id,
        success=success,
        response_time_ms=response_time_ms,
        completion_time_sec=completion_time_sec
    )
    
    # Return updated reputation data
    return reputation_manager.get_node_reputation(node_id)

@router.post("/{node_id}/uptime")
def update_node_uptime(
    node_id: str,
    uptime_hours: float = Query(..., gt=0),
    db: Session = Depends(get_db)
):
    """
    Update node uptime statistics.
    
    Args:
        node_id: The ID of the node
        uptime_hours: Hours the node has been online
        db: Database session
        
    Returns:
        Updated reputation data
    """
    reputation_manager = ReputationManager(db)
    
    # Update uptime
    reputation_manager.update_uptime(
        node_id=node_id,
        uptime_hours=uptime_hours
    )
    
    # Return updated reputation data
    return reputation_manager.get_node_reputation(node_id)

@router.post("/recalculate")
def recalculate_all_reputations(
    db: Session = Depends(get_db)
):
    """
    Recalculate reputation scores for all nodes.
    
    Args:
        db: Database session
        
    Returns:
        Number of nodes updated
    """
    reputation_manager = ReputationManager(db)
    nodes_updated = reputation_manager.recalculate_all_reputations()
    
    return {"nodes_updated": nodes_updated}

