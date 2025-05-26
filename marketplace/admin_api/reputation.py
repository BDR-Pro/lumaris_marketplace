"""
Node reputation system for the Lumaris Marketplace.

This module provides functionality to track and calculate reputation scores
for compute nodes based on their performance, reliability, and behavior.
"""

import logging
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple, Any
from sqlalchemy.orm import Session
from sqlalchemy import func

from .models import Node, Job, JobAssignment, NodeReputation

# Configure logging
logger = logging.getLogger(__name__)

class ReputationManager:
    """
    Manages reputation scores for compute nodes in the marketplace.
    
    The reputation system tracks various metrics about node performance and
    calculates a composite reputation score that can be used for job allocation
    and pricing decisions.
    """
    
    def __init__(self, db: Session):
        """
        Initialize the reputation manager.
        
        Args:
            db: Database session
        """
        self.db = db
        
        # Weights for different reputation factors
        self.weights = {
            "job_success_rate": 0.35,
            "response_time": 0.15,
            "uptime": 0.20,
            "job_completion_time": 0.15,
            "age": 0.05,
            "job_volume": 0.10
        }
        
        # Minimum number of jobs before full reputation is calculated
        self.min_jobs_for_full_reputation = 10
        
        # Default reputation for new nodes
        self.default_reputation = 0.5
    
    def get_node_reputation(self, node_id: str) -> Dict[str, Any]:
        """
        Get the reputation data for a node.
        
        Args:
            node_id: The ID of the node
            
        Returns:
            Dict containing reputation data and score
        """
        # Get or create reputation record
        reputation = self._get_or_create_reputation(node_id)
        
        # Calculate derived metrics
        job_success_rate = self._calculate_success_rate(reputation)
        uptime_percentage = self._calculate_uptime_percentage(reputation)
        avg_response_time = reputation.avg_response_time_ms
        avg_completion_time = reputation.avg_job_completion_time_sec
        
        # Calculate overall reputation score
        reputation_score = self._calculate_reputation_score(reputation)
        
        # Update the stored score if it has changed
        if abs(reputation.reputation_score - reputation_score) > 0.01:
            reputation.reputation_score = reputation_score
            self.db.commit()
        
        return {
            "node_id": node_id,
            "reputation_score": reputation_score,
            "job_success_rate": job_success_rate,
            "uptime_percentage": uptime_percentage,
            "avg_response_time_ms": avg_response_time,
            "avg_job_completion_time_sec": avg_completion_time,
            "total_jobs": reputation.successful_jobs + reputation.failed_jobs,
            "successful_jobs": reputation.successful_jobs,
            "failed_jobs": reputation.failed_jobs,
            "total_uptime_hours": reputation.total_uptime_hours,
            "first_seen": reputation.first_seen.isoformat() if reputation.first_seen else None,
            "last_updated": reputation.last_updated.isoformat()
        }
    
    def update_reputation_after_job(
        self, 
        node_id: str, 
        job_id: str, 
        success: bool, 
        response_time_ms: Optional[float] = None,
        completion_time_sec: Optional[float] = None
    ) -> float:
        """
        Update node reputation after job completion.
        
        Args:
            node_id: The ID of the node
            job_id: The ID of the job
            success: Whether the job was successful
            response_time_ms: Response time in milliseconds
            completion_time_sec: Job completion time in seconds
            
        Returns:
            Updated reputation score
        """
        # Get or create reputation record
        reputation = self._get_or_create_reputation(node_id)
        
        # Update job counts
        if success:
            reputation.successful_jobs += 1
        else:
            reputation.failed_jobs += 1
        
        # Update response time if provided
        if response_time_ms is not None:
            if reputation.avg_response_time_ms == 0:
                reputation.avg_response_time_ms = response_time_ms
            else:
                # Weighted average (more weight to recent performance)
                reputation.avg_response_time_ms = (
                    0.7 * reputation.avg_response_time_ms + 0.3 * response_time_ms
                )
        
        # Update completion time if provided
        if completion_time_sec is not None:
            if reputation.avg_job_completion_time_sec == 0:
                reputation.avg_job_completion_time_sec = completion_time_sec
            else:
                # Weighted average (more weight to recent performance)
                reputation.avg_job_completion_time_sec = (
                    0.7 * reputation.avg_job_completion_time_sec + 0.3 * completion_time_sec
                )
        
        # Calculate new reputation score
        reputation_score = self._calculate_reputation_score(reputation)
        reputation.reputation_score = reputation_score
        
        # Update last updated timestamp
        reputation.last_updated = datetime.utcnow()
        
        # Commit changes
        self.db.commit()
        
        logger.info(
            f"Updated reputation for node {node_id} after job {job_id}. "
            f"Success: {success}, New score: {reputation_score:.2f}"
        )
        
        return reputation_score
    
    def update_uptime(self, node_id: str, uptime_hours: float) -> float:
        """
        Update node uptime statistics.
        
        Args:
            node_id: The ID of the node
            uptime_hours: Hours the node has been online
            
        Returns:
            Updated reputation score
        """
        # Get or create reputation record
        reputation = self._get_or_create_reputation(node_id)
        
        # Update uptime
        reputation.total_uptime_hours += uptime_hours
        
        # Calculate new reputation score
        reputation_score = self._calculate_reputation_score(reputation)
        reputation.reputation_score = reputation_score
        
        # Update last updated timestamp
        reputation.last_updated = datetime.utcnow()
        
        # Commit changes
        self.db.commit()
        
        logger.info(
            f"Updated uptime for node {node_id}. "
            f"Added {uptime_hours:.2f} hours, New score: {reputation_score:.2f}"
        )
        
        return reputation_score
    
    def get_top_nodes(self, limit: int = 10) -> List[Dict[str, Any]]:
        """
        Get the top nodes by reputation score.
        
        Args:
            limit: Maximum number of nodes to return
            
        Returns:
            List of node reputation data
        """
        # Get top nodes by reputation score
        top_nodes = (
            self.db.query(NodeReputation)
            .order_by(NodeReputation.reputation_score.desc())
            .limit(limit)
            .all()
        )
        
        # Format results
        return [
            {
                "node_id": node.node_id,
                "reputation_score": node.reputation_score,
                "successful_jobs": node.successful_jobs,
                "failed_jobs": node.failed_jobs,
                "total_uptime_hours": node.total_uptime_hours,
                "last_updated": node.last_updated.isoformat()
            }
            for node in top_nodes
        ]
    
    def _get_or_create_reputation(self, node_id: str) -> NodeReputation:
        """
        Get or create a reputation record for a node.
        
        Args:
            node_id: The ID of the node
            
        Returns:
            NodeReputation record
        """
        # Try to get existing record
        reputation = (
            self.db.query(NodeReputation)
            .filter(NodeReputation.node_id == node_id)
            .first()
        )
        
        # Create new record if not exists
        if not reputation:
            reputation = NodeReputation(
                node_id=node_id,
                reputation_score=self.default_reputation,
                first_seen=datetime.utcnow(),
                last_updated=datetime.utcnow()
            )
            self.db.add(reputation)
            self.db.commit()
            logger.info(f"Created new reputation record for node {node_id}")
        
        return reputation
    
    def _calculate_success_rate(self, reputation: NodeReputation) -> float:
        """
        Calculate job success rate for a node.
        
        Args:
            reputation: NodeReputation record
            
        Returns:
            Success rate as a float between 0 and 1
        """
        total_jobs = reputation.successful_jobs + reputation.failed_jobs
        if total_jobs == 0:
            return 0.0
        
        return reputation.successful_jobs / total_jobs
    
    def _calculate_uptime_percentage(self, reputation: NodeReputation) -> float:
        """
        Calculate uptime percentage for a node.
        
        Args:
            reputation: NodeReputation record
            
        Returns:
            Uptime percentage as a float between 0 and 100
        """
        if not reputation.first_seen:
            return 0.0
        
        # Calculate total possible uptime since first seen
        time_since_first_seen = datetime.utcnow() - reputation.first_seen
        total_possible_hours = time_since_first_seen.total_seconds() / 3600
        
        if total_possible_hours <= 0:
            return 0.0
        
        # Cap at 100%
        return min(100.0, (reputation.total_uptime_hours / total_possible_hours) * 100)
    
    def _calculate_reputation_score(self, reputation: NodeReputation) -> float:
        """
        Calculate overall reputation score for a node.
        
        Args:
            reputation: NodeReputation record
            
        Returns:
            Reputation score as a float between 0 and 1
        """
        total_jobs = reputation.successful_jobs + reputation.failed_jobs
        
        # For nodes with few jobs, blend with default reputation
        if total_jobs < self.min_jobs_for_full_reputation:
            blend_factor = total_jobs / self.min_jobs_for_full_reputation
            base_score = self.default_reputation * (1 - blend_factor)
        else:
            blend_factor = 1.0
            base_score = 0.0
        
        # Calculate individual factor scores
        factor_scores = {}
        
        # Job success rate (higher is better)
        success_rate = self._calculate_success_rate(reputation)
        factor_scores["job_success_rate"] = success_rate
        
        # Response time (lower is better, normalize to 0-1 where 1 is best)
        if reputation.avg_response_time_ms > 0:
            # Assume 2000ms is the worst acceptable response time
            response_time_score = max(0, 1 - (reputation.avg_response_time_ms / 2000))
        else:
            response_time_score = 0.5  # Default if no data
        factor_scores["response_time"] = response_time_score
        
        # Uptime (higher is better, as percentage)
        uptime_percentage = self._calculate_uptime_percentage(reputation)
        factor_scores["uptime"] = uptime_percentage / 100  # Convert to 0-1 scale
        
        # Job completion time (lower is better, normalize to 0-1 where 1 is best)
        if reputation.avg_job_completion_time_sec > 0:
            # Normalize based on expected completion time (assume 3600s/1hr is worst acceptable)
            completion_time_score = max(0, 1 - (reputation.avg_job_completion_time_sec / 3600))
        else:
            completion_time_score = 0.5  # Default if no data
        factor_scores["job_completion_time"] = completion_time_score
        
        # Node age (higher is better, up to a point)
        if reputation.first_seen:
            age_days = (datetime.utcnow() - reputation.first_seen).days
            # Normalize to 0-1 scale, with 30 days considered mature
            age_score = min(1.0, age_days / 30)
        else:
            age_score = 0.0
        factor_scores["age"] = age_score
        
        # Job volume (higher is better, up to a point)
        # Normalize to 0-1 scale, with 100 jobs considered high volume
        job_volume_score = min(1.0, total_jobs / 100)
        factor_scores["job_volume"] = job_volume_score
        
        # Calculate weighted score
        weighted_score = sum(
            factor_scores[factor] * weight
            for factor, weight in self.weights.items()
        )
        
        # Blend with base score for nodes with few jobs
        final_score = base_score + (weighted_score * blend_factor)
        
        # Ensure score is between 0 and 1
        return max(0.0, min(1.0, final_score))
    
    def recalculate_all_reputations(self) -> int:
        """
        Recalculate reputation scores for all nodes.
        
        Returns:
            Number of nodes updated
        """
        # Get all reputation records
        reputations = self.db.query(NodeReputation).all()
        
        # Recalculate scores
        for reputation in reputations:
            reputation.reputation_score = self._calculate_reputation_score(reputation)
            reputation.last_updated = datetime.utcnow()
        
        # Commit changes
        self.db.commit()
        
        logger.info(f"Recalculated reputation scores for {len(reputations)} nodes")
        
        return len(reputations)

