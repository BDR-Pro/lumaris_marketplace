# pylint: disable=relative-beyond-top-level , line-too-long
"""URL configuration for the marketplace app."""
from django.contrib.auth import views as auth_views
from django.urls import path

from .views import (
    admin_dashboard,
    api_jobs,
    api_nodes,
    buyer_dashboard,
    dashboard,
    jobs_list,
    landing_page,
    nodes_list,
    seller_dashboard,
    signup_view,
    submit_job,
)

urlpatterns = [
    # Main pages
    path("", landing_page, name="landing"),
    path("dashboard/", dashboard, name="dashboard"),
    # Authentication
    path(
        "login/",
        auth_views.LoginView.as_view(template_name="registration/login.html"),
        name="login",
    ),
    path("logout/", auth_views.LogoutView.as_view(), name="logout"),
    path("signup/", signup_view, name="signup"),
    # Dashboards
    path("buyer/", buyer_dashboard, name="buyer_dashboard"),
    path("seller/", seller_dashboard, name="seller_dashboard"),
    path("admin/", admin_dashboard, name="admin_dashboard"),
    # Marketplace
    path("jobs/", jobs_list, name="jobs_list"),
    path("jobs/submit/", submit_job, name="submit_job"),
    path("nodes/", nodes_list, name="nodes_list"),
    # API endpoints
    path("api/nodes/", api_nodes, name="api_nodes"),
    path("api/jobs/", api_jobs, name="api_jobs"),
]
