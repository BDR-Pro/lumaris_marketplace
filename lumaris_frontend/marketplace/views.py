"""views.py - Marketplace app views for Lumaris"""

import requests
from django.conf import settings
from django.contrib import messages
from django.contrib.auth import login
from django.contrib.auth.decorators import login_required
from django.contrib.auth.forms import UserCreationForm
from django.http import JsonResponse
from django.shortcuts import redirect, render

from marketplace.forms import JobSubmissionForm
from marketplace.models import Job, Node


def landing_page(request):
    """Landing page with hero section and features"""
    features = [
        (
            "fas fa-shield-alt",
            "Secure Isolation",
            "Every job runs in isolated Firecracker microVMs...",
        ),
        (
            "fas fa-network-wired",
            "Distributed Computing",
            "Split large tasks across nodes...",
        ),
        (
            "fas fa-chart-line",
            "Real-time Monitoring",
            "Track nodes with live dashboards...",
        ),
        (
            "fas fa-balance-scale",
            "Smart Matchmaking",
            "Advanced algorithms match jobs...",
        ),
        ("fas fa-coins", "Fair Pricing", "Dynamic pricing ensures fair compensation."),
        ("fas fa-rocket", "High Performance", "Built with Rust for performance."),
    ]
    context = {
        "total_nodes": Node.objects.filter(status="online").count(),
        "total_jobs": Job.objects.count(),
        "completed_jobs": Job.objects.filter(status="completed").count(),
        "features": features,
    }
    return render(request, "landing.html", context)


def dashboard(request):
    """Main dashboard - redirects based on user type"""
    if request.user.is_staff:
        return redirect("admin_dashboard")
    return redirect("buyer_dashboard")


@login_required
def buyer_dashboard(request):
    """Dashboard for job buyers"""
    jobs = Job.objects.filter(owner=request.user).order_by("-submitted_at")
    context = {
        "jobs": jobs,
        "pending_jobs": jobs.filter(status="pending").count(),
        "running_jobs": jobs.filter(status="running").count(),
        "completed_jobs": jobs.filter(status="completed").count(),
    }
    return render(request, "dashboard/buyer.html", context)


@login_required
def seller_dashboard(request):
    """Dashboard for node sellers"""
    # This would integrate with your Rust GUI nodes
    nodes = Node.objects.all()
    context = {
        "nodes": nodes,
        "total_earnings": sum(node.funds_earned for node in nodes),
    }
    return render(request, "dashboard/seller.html", context)


@login_required
def admin_dashboard(request):
    """Admin dashboard with system overview"""
    if not request.user.is_staff:
        return redirect("buyer_dashboard")
    context = {
        "total_nodes": Node.objects.count(),
        "online_nodes": Node.objects.filter(status="online").count(),
        "total_jobs": Job.objects.count(),
        "pending_jobs": Job.objects.filter(status="pending").count(),
        "running_jobs": Job.objects.filter(status="running").count(),
        "recent_jobs": Job.objects.order_by("-submitted_at")[:10],
        "recent_nodes": Node.objects.order_by("-last_seen")[:10],
    }
    return render(request, "dashboard/admin.html", context)


@login_required
def submit_job(request):
    """Job submission form"""
    if request.method == "POST":
        form = JobSubmissionForm(request.POST)
        if form.is_valid():
            job = form.save(commit=False)
            job.owner = request.user
            job.save()
            # Submit job to your FastAPI backend
            try:
                job_data = {
                    "title": job.title,
                    "description": job.description,
                    "cpu_cores_required": job.cpu_cores_required,
                    "memory_mb_required": job.memory_mb_required,
                    "expected_duration_sec": job.expected_duration_sec,
                    "priority": job.priority,
                }
                response = requests.post(
                    f"{settings.LUMARIS_API_URL}/jobs/", json=job_data, timeout=10
                )
                if response.status_code == 200:
                    messages.success(request, "Job submitted successfully!")
                else:
                    messages.warning(
                        request,
                        "Job saved locally but failed to submit to processing queue.",
                    )
            except requests.RequestException as e:
                messages.error(request, f"Error submitting job: {str(e)}")
            return redirect("buyer_dashboard")
    else:
        form = JobSubmissionForm()
    return render(request, "marketplace/submit_job.html", {"form": form})


def jobs_list(request):
    """Public jobs listing"""
    jobs = Job.objects.filter(status__in=["pending", "running"]).order_by(
        "-submitted_at"
    )
    return render(request, "marketplace/jobs.html", {"jobs": jobs})


def nodes_list(request):
    """Public nodes listing"""
    nodes = Node.objects.filter(status="online").order_by("-last_seen")
    return render(request, "marketplace/nodes.html", {"nodes": nodes})


def api_nodes(_request):
    """API endpoint to sync with your FastAPI backend"""
    try:
        response = requests.get(f"{settings.LUMARIS_API_URL}/nodes/", timeout=10)
        if response.status_code == 200:
            return JsonResponse(response.json(), safe=False)
        return JsonResponse({"error": "Failed to fetch nodes"}, status=500)
    except requests.RequestException as e:
        return JsonResponse({"error": str(e)}, status=500)


def api_jobs(_request):
    """API endpoint to sync with your FastAPI backend"""
    try:
        response = requests.get(f"{settings.LUMARIS_API_URL}/jobs/", timeout=10)
        if response.status_code == 200:
            return JsonResponse(response.json(), safe=False)
        return JsonResponse({"error": "Failed to fetch jobs"}, status=500)
    except requests.RequestException as e:
        return JsonResponse({"error": str(e)}, status=500)


def signup_view(request):
    """User signup view and sign in directly"""
    # Check if user is already authenticated
    if request.user.is_authenticated:
        return redirect("dashboard")
    if request.method == "POST":
        form = UserCreationForm(request.POST)
        if form.is_valid():
            user = form.save()
            messages.success(
                request, "Account created successfully! You can now log in."
            )
            # Automatically log in the user after signup
            login(request, user)
            return redirect("dashboard")
    else:
        form = UserCreationForm()
    return render(request, "registration/signup.html", {"form": form})
