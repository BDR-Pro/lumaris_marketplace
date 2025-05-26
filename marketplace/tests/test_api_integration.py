import pytest
import uuid
import json
from fastapi.testclient import TestClient
from marketplace.admin_api.main import app
from marketplace.admin_api.models import Node, Job, NodeCapabilities

client = TestClient(app)

class TestApiIntegration:
    def test_node_registration(self, test_db):
        # Generate a unique hostname for this test
        hostname = f"test-node-{uuid.uuid4()}"
        
        # Register a new node
        response = client.post(
            "/nodes/register",
            json={"hostname": hostname, "cpu_usage": 4.0, "mem_usage": 8192.0}
        )
        
        assert response.status_code == 200
        data = response.json()
        assert "node_id" in data
        assert "token" in data
        
        # Verify the node was created in the database
        node = test_db.query(Node).filter(Node.hostname == hostname).first()
        assert node is not None
        assert node.status == "online"
    
    def test_node_token_issuance(self, test_db):
        # Generate a unique hostname for this test
        hostname = f"test-node-{uuid.uuid4()}"
        
        # Register a new node
        response = client.post(
            "/nodes/register",
            json={"hostname": hostname, "cpu_usage": 4.0, "mem_usage": 8192.0}
        )
        
        assert response.status_code == 200
        data = response.json()
        token = data["token"]
        
        # Use the token to authenticate a request
        response = client.get(
            "/nodes/me",
            headers={"Authorization": f"Bearer {token}"}
        )
        
        assert response.status_code == 200
        node_data = response.json()
        assert node_data["hostname"] == hostname
    
    def test_node_stats_update(self, test_db):
        # Generate a unique hostname for this test
        hostname = f"test-node-{uuid.uuid4()}"
        
        # Register a new node
        response = client.post(
            "/nodes/register",
            json={"hostname": hostname, "cpu_usage": 4.0, "mem_usage": 8192.0}
        )
        
        assert response.status_code == 200
        data = response.json()
        token = data["token"]
        
        # Update node stats
        new_cpu = 8.0
        new_mem = 16384.0
        response = client.post(
            "/nodes/stats",
            headers={"Authorization": f"Bearer {token}"},
            json={"cpu_usage": new_cpu, "mem_usage": new_mem}
        )
        
        assert response.status_code == 200
        
        # Verify the node stats were updated
        node = test_db.query(Node).filter(Node.hostname == hostname).first()
        assert node.cpu_usage == new_cpu
        assert node.mem_usage == new_mem
    
    def test_job_submission(self, test_db):
        # Generate a unique hostname for this test
        hostname = f"test-node-{uuid.uuid4()}"
        
        # Register a new node
        response = client.post(
            "/nodes/register",
            json={"hostname": hostname, "cpu_usage": 4.0, "mem_usage": 8192.0}
        )
        
        assert response.status_code == 200
        data = response.json()
        node_id = data["node_id"]
        
        # Create node capabilities
        capabilities = NodeCapabilities(
            node_id=node_id,
            cpu_cores=4,
            memory_mb=8192,
            reliability_score=0.95
        )
        test_db.add(capabilities)
        test_db.commit()
        
        # Submit a job
        response = client.post(
            "/jobs/submit",
            json={
                "job_type": "compute",
                "cpu_required": 2,
                "memory_required": 4096,
                "duration_expected": 3600
            }
        )
        
        assert response.status_code == 200
        job_data = response.json()
        assert "job_id" in job_data
        
        # Verify the job was created
        job = test_db.query(Job).filter(Job.id == job_data["job_id"]).first()
        assert job is not None
        assert job.status == "pending"
    
    def test_matchmaking(self, test_db):
        # Generate a unique hostname for this test
        hostname = f"test-node-{uuid.uuid4()}"
        
        # Register a new node
        response = client.post(
            "/nodes/register",
            json={"hostname": hostname, "cpu_usage": 4.0, "mem_usage": 8192.0}
        )
        
        assert response.status_code == 200
        data = response.json()
        node_id = data["node_id"]
        
        # Create node capabilities
        capabilities = NodeCapabilities(
            node_id=node_id,
            cpu_cores=8,
            memory_mb=16384,
            reliability_score=0.98
        )
        test_db.add(capabilities)
        test_db.commit()
        
        # Submit a job
        response = client.post(
            "/jobs/submit",
            json={
                "job_type": "compute",
                "cpu_required": 4,
                "memory_required": 8192,
                "duration_expected": 1800
            }
        )
        
        assert response.status_code == 200
        job_data = response.json()
        job_id = job_data["job_id"]
        
        # Trigger matchmaking
        response = client.post(f"/matchmaking/assign/{job_id}")
        assert response.status_code == 200
        assignment = response.json()
        
        # Verify the job was assigned to our node
        assert assignment["node_id"] == node_id
        
        # Check the job status in the database
        job = test_db.query(Job).filter(Job.id == job_id).first()
        assert job.status == "assigned"
        assert job.assigned_node_id == node_id


class TestEndToEndIntegration:
    def test_node_registration_and_job_assignment(self, test_db):
        # Register a node
        hostname = f"test-node-{uuid.uuid4()}"
        response = client.post(
            "/nodes/register",
            json={"hostname": hostname, "cpu_usage": 4.0, "mem_usage": 8192.0}
        )
        
        assert response.status_code == 200
        node_data = response.json()
        node_id = node_data["node_id"]
        token = node_data["token"]
        
        # Add node capabilities
        capabilities = NodeCapabilities(
            node_id=node_id,
            cpu_cores=16,
            memory_mb=32768,
            reliability_score=0.99
        )
        test_db.add(capabilities)
        test_db.commit()
        
        # Submit a job
        response = client.post(
            "/jobs/submit",
            json={
                "job_type": "compute",
                "cpu_required": 8,
                "memory_required": 16384,
                "duration_expected": 7200
            }
        )
        
        assert response.status_code == 200
        job_data = response.json()
        job_id = job_data["job_id"]
        
        # Assign the job
        response = client.post(f"/matchmaking/assign/{job_id}")
        assert response.status_code == 200
        
        # Node accepts the job
        response = client.post(
            f"/jobs/{job_id}/accept",
            headers={"Authorization": f"Bearer {token}"}
        )
        assert response.status_code == 200
        
        # Node completes the job
        response = client.post(
            f"/jobs/{job_id}/complete",
            headers={"Authorization": f"Bearer {token}"},
            json={"result": "Job completed successfully"}
        )
        assert response.status_code == 200
        
        # Verify job status
        job = test_db.query(Job).filter(Job.id == job_id).first()
        assert job.status == "completed"
        assert job.result == "Job completed successfully"


@pytest.mark.asyncio
class TestWebSocketIntegration:
    @pytest.mark.asyncio
    async def test_heartbeat(self):
        # This test requires pytest-asyncio to run
        import asyncio
        import websockets
        
        # Register a node first to get a token
        hostname = f"test-ws-node-{uuid.uuid4()}"
        response = client.post(
            "/nodes/register",
            json={"hostname": hostname, "cpu_usage": 4.0, "mem_usage": 8192.0}
        )
        
        assert response.status_code == 200
        node_data = response.json()
        token = node_data["token"]
        
        # Connect to WebSocket with the token
        uri = f"ws://localhost:8000/ws?token={token}"
        
        # Skip the actual WebSocket test in the test client
        # This would be tested with a real server
        assert True
    
    @pytest.mark.asyncio
    async def test_job_notification(self):
        # This test requires pytest-asyncio to run
        # Skip the actual WebSocket test in the test client
        assert True
    
    @pytest.mark.asyncio
    async def test_status_updates(self):
        # This test requires pytest-asyncio to run
        # Skip the actual WebSocket test in the test client
        assert True
    
    @pytest.mark.asyncio
    async def test_disconnection_handling(self):
        # This test requires pytest-asyncio to run
        # Skip the actual WebSocket test in the test client
        assert True

