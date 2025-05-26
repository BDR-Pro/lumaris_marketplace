"""
Tests for the node reputation system.
"""

import pytest
from datetime import datetime, timedelta
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

from admin_api.main import app
from admin_api.database import get_db
from admin_api.models import Base, NodeReputation
from admin_api.reputation import ReputationManager

# Create in-memory SQLite database for testing
SQLALCHEMY_DATABASE_URL = "sqlite:///:memory:"
engine = create_engine(
    SQLALCHEMY_DATABASE_URL,
    connect_args={"check_same_thread": False},
    poolclass=StaticPool,
)
TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

# Create tables
Base.metadata.create_all(bind=engine)

# Override the get_db dependency
def override_get_db():
    db = TestingSessionLocal()
    try:
        yield db
    finally:
        db.close()

app.dependency_overrides[get_db] = override_get_db

# Test client
@pytest.fixture
def client():
    return TestClient(app)

# Test database session
@pytest.fixture
def db():
    db = TestingSessionLocal()
    try:
        yield db
    finally:
        db.close()

def test_create_reputation(db):
    """Test creating a new reputation record."""
    # Create reputation manager
    reputation_manager = ReputationManager(db)
    
    # Get reputation for a new node
    node_id = "test_node_1"
    reputation = reputation_manager.get_node_reputation(node_id)
    
    # Check that a record was created
    assert reputation["node_id"] == node_id
    assert reputation["reputation_score"] == 0.5  # Default score
    assert reputation["total_jobs"] == 0
    
    # Check database record
    db_record = db.query(NodeReputation).filter(NodeReputation.node_id == node_id).first()
    assert db_record is not None
    assert db_record.reputation_score == 0.5

def test_update_reputation_after_job(db):
    """Test updating reputation after job completion."""
    # Create reputation manager
    reputation_manager = ReputationManager(db)
    
    # Get reputation for a new node
    node_id = "test_node_2"
    
    # Update reputation after successful job
    reputation_manager.update_reputation_after_job(
        node_id=node_id,
        job_id="job1",
        success=True,
        response_time_ms=100,
        completion_time_sec=60
    )
    
    # Get updated reputation
    reputation = reputation_manager.get_node_reputation(node_id)
    
    # Check that reputation was updated
    assert reputation["successful_jobs"] == 1
    assert reputation["failed_jobs"] == 0
    assert reputation["avg_response_time_ms"] == 100
    assert reputation["avg_job_completion_time_sec"] == 60
    
    # Update reputation after failed job
    reputation_manager.update_reputation_after_job(
        node_id=node_id,
        job_id="job2",
        success=False,
        response_time_ms=200,
        completion_time_sec=120
    )
    
    # Get updated reputation
    reputation = reputation_manager.get_node_reputation(node_id)
    
    # Check that reputation was updated
    assert reputation["successful_jobs"] == 1
    assert reputation["failed_jobs"] == 1
    assert reputation["avg_response_time_ms"] > 100  # Should be weighted average
    assert reputation["avg_job_completion_time_sec"] > 60  # Should be weighted average

def test_update_uptime(db):
    """Test updating node uptime."""
    # Create reputation manager
    reputation_manager = ReputationManager(db)
    
    # Get reputation for a new node
    node_id = "test_node_3"
    
    # Update uptime
    reputation_manager.update_uptime(node_id=node_id, uptime_hours=10)
    
    # Get updated reputation
    reputation = reputation_manager.get_node_reputation(node_id)
    
    # Check that uptime was updated
    assert reputation["total_uptime_hours"] == 10
    
    # Update uptime again
    reputation_manager.update_uptime(node_id=node_id, uptime_hours=5)
    
    # Get updated reputation
    reputation = reputation_manager.get_node_reputation(node_id)
    
    # Check that uptime was updated
    assert reputation["total_uptime_hours"] == 15

def test_get_top_nodes(db):
    """Test getting top nodes by reputation."""
    # Create reputation manager
    reputation_manager = ReputationManager(db)
    
    # Create several nodes with different reputations
    nodes = [
        {"node_id": "top_node_1", "success": True, "jobs": 10},
        {"node_id": "top_node_2", "success": False, "jobs": 10},
        {"node_id": "top_node_3", "success": True, "jobs": 5},
    ]
    
    for node in nodes:
        # Update reputation for each node
        for i in range(node["jobs"]):
            reputation_manager.update_reputation_after_job(
                node_id=node["node_id"],
                job_id=f"job_{node['node_id']}_{i}",
                success=node["success"],
                response_time_ms=100,
                completion_time_sec=60
            )
    
    # Get top nodes
    top_nodes = reputation_manager.get_top_nodes(limit=3)
    
    # Check that nodes are returned in correct order
    assert len(top_nodes) == 3
    
    # The node with all successful jobs should have highest reputation
    assert top_nodes[0]["node_id"] == "top_node_1"
    
    # The node with all failed jobs should have lowest reputation
    assert top_nodes[2]["node_id"] == "top_node_2"

def test_reputation_api_endpoints(client, db):
    """Test reputation API endpoints."""
    # Create a node with reputation data
    reputation_manager = ReputationManager(db)
    node_id = "api_test_node"
    
    # Update reputation
    reputation_manager.update_reputation_after_job(
        node_id=node_id,
        job_id="job1",
        success=True,
        response_time_ms=100,
        completion_time_sec=60
    )
    
    # Test get node reputation endpoint
    response = client.get(f"/reputation/{node_id}")
    assert response.status_code == 200
    data = response.json()
    assert data["node_id"] == node_id
    assert data["successful_jobs"] == 1
    
    # Test update reputation after job endpoint
    response = client.post(
        f"/reputation/{node_id}/job",
        params={
            "job_id": "job2",
            "success": True,
            "response_time_ms": 150,
            "completion_time_sec": 90
        }
    )
    assert response.status_code == 200
    data = response.json()
    assert data["successful_jobs"] == 2
    
    # Test update uptime endpoint
    response = client.post(
        f"/reputation/{node_id}/uptime",
        params={"uptime_hours": 5}
    )
    assert response.status_code == 200
    data = response.json()
    assert data["total_uptime_hours"] == 5
    
    # Test get top nodes endpoint
    response = client.get("/reputation/")
    assert response.status_code == 200
    data = response.json()
    assert len(data) > 0
    
    # Test recalculate all reputations endpoint
    response = client.post("/reputation/recalculate")
    assert response.status_code == 200
    data = response.json()
    assert data["nodes_updated"] > 0

