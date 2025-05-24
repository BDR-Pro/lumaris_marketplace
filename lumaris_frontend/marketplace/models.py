"""
This module defines the database models for the marketplace application,
specifically for managing computational nodes and jobs
Each model is designed to facilitate the scheduling,
execution, and monitoring of jobs across distributed nodes.


"""

from django.contrib.auth.models import User
from django.db import models


class Node(models.Model):
    """Represents a computational node, tracking its resources, status, and earnings."""

    node_id = models.CharField(max_length=100, unique=True)
    hostname = models.CharField(max_length=255)
    cpu_cores = models.IntegerField()
    memory_mb = models.IntegerField()
    cpu_usage = models.FloatField(default=0.0)
    memory_usage = models.FloatField(default=0.0)
    funds_earned = models.DecimalField(max_digits=10, decimal_places=2, default=0.00)
    reliability_score = models.FloatField(default=1.0)
    status = models.CharField(max_length=20, default="offline")
    last_seen = models.DateTimeField(auto_now=True)
    created_at = models.DateTimeField(auto_now_add=True)

    def __str__(self):
        return f"{self.hostname} ({self.node_id})"


class Job(models.Model):
    """Job submitted by a user, including its requirements, status, and assignment."""

    STATUS_CHOICES = [
        ("pending", "Pending"),
        ("running", "Running"),
        ("completed", "Completed"),
        ("failed", "Failed"),
        ("cancelled", "Cancelled"),
    ]
    PRIORITY_CHOICES = [
        (1, "High"),
        (2, "Medium"),
        (3, "Low"),
    ]

    title = models.CharField(max_length=255)
    description = models.TextField()
    owner = models.ForeignKey(User, on_delete=models.CASCADE)
    cpu_cores_required = models.IntegerField()
    memory_mb_required = models.IntegerField()
    expected_duration_sec = models.IntegerField()
    priority = models.IntegerField(choices=PRIORITY_CHOICES, default=2)
    status = models.CharField(max_length=20, choices=STATUS_CHOICES, default="pending")
    assigned_node = models.ForeignKey(
        Node, on_delete=models.SET_NULL, null=True, blank=True
    )
    submitted_at = models.DateTimeField(auto_now_add=True)
    started_at = models.DateTimeField(null=True, blank=True)
    completed_at = models.DateTimeField(null=True, blank=True)
    cost = models.DecimalField(max_digits=10, decimal_places=2, default=0.00)

    def __str__(self):
        return f"{self.title} - {self.status}"


class JobRequirement(models.Model):
    """Requirements for a job, such as CPU, memory, and duration."""

    job = models.OneToOneField(
        Job, on_delete=models.CASCADE, related_name="requirements"
    )
    cpu_cores = models.IntegerField()
    memory_mb = models.IntegerField()
    expected_duration_sec = models.IntegerField()
    priority = models.IntegerField(default=1)


class NodeCapability(models.Model):
    """Describes the capabilities of a node, including available resources and reliability."""

    node = models.OneToOneField(
        Node, on_delete=models.CASCADE, related_name="capabilities"
    )
    cpu_cores = models.IntegerField()
    memory_mb = models.IntegerField()
    reliability_score = models.FloatField(default=1.0)


class JobAssignment(models.Model):
    """Tracks the assignment of jobs to nodes, including assignment and completion times"""

    job = models.ForeignKey(Job, on_delete=models.CASCADE)
    node = models.ForeignKey(Node, on_delete=models.CASCADE)
    assigned_at = models.DateTimeField(auto_now_add=True)
    started_at = models.DateTimeField(null=True, blank=True)
    completed_at = models.DateTimeField(null=True, blank=True)
    status = models.CharField(max_length=20, default="assigned")
    cost = models.DecimalField(max_digits=10, decimal_places=2, default=0.00)
