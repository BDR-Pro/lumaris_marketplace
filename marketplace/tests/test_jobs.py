"""Tests for the /jobs endpoints of the Admin API."""

def test_list_jobs_returns_empty_list_initially(client):
    """Test that the jobs list is initially empty."""
    response = client.get("/jobs/")
    assert response.status_code == 200
    assert response.json() == []


def test_submit_job_creates_new_job(client):
    """Test that submitting a new job successfully creates it."""
    job_data = {
        "node_id": 1,
        "status": "pending",
        "duration": 3.5,
        "cost": 10.0,
    }
    response = client.post("/jobs/", json=job_data)
    assert response.status_code == 200
    job = response.json()
    assert job["node_id"] == job_data["node_id"]
    assert job["status"] == job_data["status"]


def test_list_jobs_returns_created_job(client):
    """Test that the created job appears in the jobs list."""
    job_data = {
        "node_id": 2,
        "status": "queued",
        "duration": 1.2,
        "cost": 5.0,
    }
    response = client.post("/jobs/", json=job_data)
    assert response.status_code == 200
    response = client.get("/jobs/")
    jobs = response.json()
    assert isinstance(jobs, list)
    assert len(jobs) == 1
    assert jobs[0]["node_id"] == job_data["node_id"]
