# pylint: disable=unused-argument , redefined-outer-name
"""Tests for marketplace models using pytest."""

import pytest
from django.utils import timezone
from django.contrib.auth.models import User
from marketplace.models import (
    Node,
    Job,
    JobRequirement,
    NodeCapability,
    JobAssignment
)

# ------------------- Fixtures -------------------

@pytest.fixture
def user_fixture(db):
    """Fixture to create a test user."""
    return User.objects.create_user(username="tester", password="password")


@pytest.fixture
def node_fixture(db):
    """Fixture to create a test node."""
    return Node.objects.create(
        node_id="node-001",
        hostname="node1",
        cpu_cores=8,
        memory_mb=16384
    )


@pytest.fixture
def job_fixture(db, user_fixture):
    """Fixture to create a test job."""
    return Job.objects.create(
        title="Test Job",
        description="A job to test.",
        owner=user_fixture,
        cpu_cores_required=4,
        memory_mb_required=2048,
        expected_duration_sec=3600,
        priority=1
    )


# ------------------- Tests -------------------

def test_node_creation_defaults(node_fixture):
    """Test node creation and default field values."""
    assert node_fixture.status == "offline"
    assert node_fixture.cpu_usage == 0.0
    assert node_fixture.memory_usage == 0.0
    assert node_fixture.funds_earned == 0.00


def test_job_creation_fields(job_fixture):
    """Test job fields and defaults."""
    assert job_fixture.title == "Test Job"
    assert job_fixture.status == "pending"
    assert job_fixture.priority == 1

def test_job_requirement_creation(job_fixture):

    """Test job requirements relationship."""
    requirement = JobRequirement.objects.create(
        job=job_fixture,
        cpu_cores=4,
        memory_mb=2048,
        expected_duration_sec=3600,
        priority=1
    )
    assert requirement.cpu_cores == 4
    assert requirement.job == job_fixture


def test_node_capability_creation(node_fixture):
    """Test node capabilities relationship."""
    capability = NodeCapability.objects.create(
        node=node_fixture,
        cpu_cores=8,
        memory_mb=16384,
        reliability_score=0.95
    )
    assert capability.reliability_score == 0.95
    assert capability.node == node_fixture


def test_job_assignment_creation(job_fixture, node_fixture):
    """Test assignment of a job to a node."""
    assignment = JobAssignment.objects.create(
        job=job_fixture,
        node=node_fixture,
        status="assigned",
        cost=0.50
    )
    assert assignment.status == "assigned"
    assert assignment.node == node_fixture
    assert assignment.job == job_fixture
    assert assignment.cost == 0.50


def test_job_status_transitions(job_fixture):
    """Test transition of job status from running to completed."""
    job_fixture.status = "running"
    job_fixture.started_at = timezone.now()
    job_fixture.save()
    assert job_fixture.status == "running"

    job_fixture.status = "completed"
    job_fixture.completed_at = timezone.now()
    job_fixture.save()
    assert job_fixture.status == "completed"


def test_job_assignment_timestamps(job_fixture, node_fixture):
    """Test timestamps on job assignment model."""
    assignment = JobAssignment.objects.create(
        job=job_fixture,
        node=node_fixture,
        status="assigned",
        cost=0.75
    )
    assert assignment.assigned_at is not None
    assert assignment.started_at is None
    assert assignment.completed_at is None


def test_node_last_seen_update(node_fixture):
    """Test that last_seen updates on save."""
    before = node_fixture.last_seen
    node_fixture.cpu_usage = 50.0
    node_fixture.save()
    assert node_fixture.last_seen >= before


def test_multiple_jobs_per_user(user_fixture):
    """Test that a user can own multiple jobs."""
    job1 = Job.objects.create(
        title="Job One",
        description="Job 1",
        owner=user_fixture,
        cpu_cores_required=2,
        memory_mb_required=1024,
        expected_duration_sec=1800,
        priority=2
    )
    job2 = Job.objects.create(
        title="Job Two",
        description="Job 2",
        owner=user_fixture,
        cpu_cores_required=4,
        memory_mb_required=2048,
        expected_duration_sec=3600,
        priority=3
    )
    assert job1.owner == job2.owner
    assert Job.objects.filter(owner=user_fixture).count() == 2


def test_reassign_job_to_another_node(job_fixture, node_fixture, db):
    """Test job reassignment from one node to another."""
    node2 = Node.objects.create(
        node_id="node-002",
        hostname="node2",
        cpu_cores=16,
        memory_mb=32768
    )
    job_fixture.assigned_node = node_fixture
    job_fixture.save()
    assert job_fixture.assigned_node == node_fixture

    job_fixture.assigned_node = node2
    job_fixture.save()
    assert job_fixture.assigned_node == node2
