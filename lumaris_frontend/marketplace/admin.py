"""Admin configuration for the marketplace app in Django."""

# pylint: disable=relative-beyond-top-level
from django.contrib import admin

from .models import Job, JobAssignment, JobRequirement, Node, NodeCapability


@admin.register(Node)
class NodeAdmin(admin.ModelAdmin):
    """Admin interface for managing nodes in the marketplace."""

    list_display = [
        "node_id",
        "hostname",
        "status",
        "cpu_usage",
        "memory_usage",
        "funds_earned",
        "reliability_score",
        "last_seen",
    ]
    list_filter = ["status", "last_seen"]
    search_fields = ["node_id", "hostname"]
    readonly_fields = ["created_at", "last_seen"]


@admin.register(Job)
class JobAdmin(admin.ModelAdmin):
    """Admin interface for managing jobs in the marketplace."""

    list_display = [
        "title",
        "owner",
        "status",
        "priority",
        "cpu_cores_required",
        "memory_mb_required",
        "submitted_at",
        "assigned_node",
    ]
    list_filter = ["status", "priority", "submitted_at"]
    search_fields = ["title", "description", "owner__username"]
    readonly_fields = ["submitted_at", "started_at", "completed_at"]


admin.site.register(JobRequirement)
admin.site.register(NodeCapability)
admin.site.register(JobAssignment)
