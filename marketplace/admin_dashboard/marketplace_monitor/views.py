from django.shortcuts import render, redirect, get_object_or_404
from django.http import JsonResponse, HttpResponse
from django.views.decorators.http import require_http_methods
from django.contrib.auth.decorators import login_required
from django.views.decorators.csrf import csrf_exempt, ensure_csrf_cookie
from django.utils import timezone
from django.db.models import Count, Sum, Avg, F, Q
from django.core.paginator import Paginator
from django.conf import settings

import json
import asyncio
import logging
from datetime import datetime, timedelta
from uuid import uuid4

from .models import Node, VirtualMachine, Transaction, Metric
from .api_client import api_client
from .ws_client import ws_client

logger = logging.getLogger(__name__)

# Dashboard views
@login_required
def dashboard(request):
    """Main dashboard view."""
    return render(request, 'marketplace_monitor/dashboard.html', {
        'page_title': 'Dashboard',
        'ws_url': settings.WS_URL,
    })

@login_required
def nodes(request):
    """View for managing nodes."""
    nodes_list = Node.objects.all()
    paginator = Paginator(nodes_list, 10)
    page_number = request.GET.get('page', 1)
    nodes = paginator.get_page(page_number)
    
    return render(request, 'marketplace_monitor/nodes.html', {
        'page_title': 'Nodes',
        'nodes': nodes,
    })

@login_required
def node_detail(request, node_id):
    """View for node details."""
    node = get_object_or_404(Node, node_id=node_id)
    vms = node.vms.all()
    
    # Get CPU metrics for the last 24 hours
    cpu_metrics = Metric.objects.filter(
        node=node,
        metric_type='node_cpu',
        timestamp__gte=timezone.now() - timedelta(hours=24)
    ).order_by('timestamp')
    
    # Get memory metrics for the last 24 hours
    memory_metrics = Metric.objects.filter(
        node=node,
        metric_type='node_memory',
        timestamp__gte=timezone.now() - timedelta(hours=24)
    ).order_by('timestamp')
    
    return render(request, 'marketplace_monitor/node_detail.html', {
        'page_title': f'Node: {node.node_id}',
        'node': node,
        'vms': vms,
        'cpu_metrics': cpu_metrics,
        'memory_metrics': memory_metrics,
    })

@login_required
def vms(request):
    """View for managing VMs."""
    vms_list = VirtualMachine.objects.all()
    paginator = Paginator(vms_list, 10)
    page_number = request.GET.get('page', 1)
    vms = paginator.get_page(page_number)
    
    return render(request, 'marketplace_monitor/vms.html', {
        'page_title': 'Virtual Machines',
        'vms': vms,
    })

@login_required
def vm_detail(request, vm_id):
    """View for VM details."""
    vm = get_object_or_404(VirtualMachine, vm_id=vm_id)
    transactions = vm.transactions.all()
    
    # Get CPU metrics for the last 24 hours
    cpu_metrics = Metric.objects.filter(
        vm=vm,
        metric_type='vm_cpu',
        timestamp__gte=timezone.now() - timedelta(hours=24)
    ).order_by('timestamp')
    
    # Get memory metrics for the last 24 hours
    memory_metrics = Metric.objects.filter(
        vm=vm,
        metric_type='vm_memory',
        timestamp__gte=timezone.now() - timedelta(hours=24)
    ).order_by('timestamp')
    
    return render(request, 'marketplace_monitor/vm_detail.html', {
        'page_title': f'VM: {vm.vm_id}',
        'vm': vm,
        'transactions': transactions,
        'cpu_metrics': cpu_metrics,
        'memory_metrics': memory_metrics,
    })

@login_required
def transactions(request):
    """View for managing transactions."""
    transactions_list = Transaction.objects.all()
    paginator = Paginator(transactions_list, 10)
    page_number = request.GET.get('page', 1)
    transactions = paginator.get_page(page_number)
    
    return render(request, 'marketplace_monitor/transactions.html', {
        'page_title': 'Transactions',
        'transactions': transactions,
    })

@login_required
def marketplace_stats(request):
    """View for marketplace statistics."""
    # Get active nodes count
    active_nodes_count = Node.objects.filter(status='online').count()
    
    # Get active VMs count
    active_vms_count = VirtualMachine.objects.filter(status='running').count()
    
    # Get total transaction volume
    total_volume = Transaction.objects.filter(
        status='completed',
        transaction_type='payment'
    ).aggregate(total=Sum('amount'))['total'] or 0
    
    # Get marketplace volume metrics for the last 7 days
    volume_metrics = Metric.objects.filter(
        metric_type='marketplace_volume',
        timestamp__gte=timezone.now() - timedelta(days=7)
    ).order_by('timestamp')
    
    # Get active nodes metrics for the last 7 days
    nodes_metrics = Metric.objects.filter(
        metric_type='active_nodes',
        timestamp__gte=timezone.now() - timedelta(days=7)
    ).order_by('timestamp')
    
    # Get active VMs metrics for the last 7 days
    vms_metrics = Metric.objects.filter(
        metric_type='active_vms',
        timestamp__gte=timezone.now() - timedelta(days=7)
    ).order_by('timestamp')
    
    return render(request, 'marketplace_monitor/marketplace_stats.html', {
        'page_title': 'Marketplace Statistics',
        'active_nodes_count': active_nodes_count,
        'active_vms_count': active_vms_count,
        'total_volume': total_volume,
        'volume_metrics': volume_metrics,
        'nodes_metrics': nodes_metrics,
        'vms_metrics': vms_metrics,
    })

# API views
@login_required
@require_http_methods(["GET"])
def api_nodes(request):
    """API endpoint for nodes."""
    nodes = Node.objects.all().values()
    return JsonResponse(list(nodes), safe=False)

@login_required
@require_http_methods(["GET"])
def api_node_detail(request, node_id):
    """API endpoint for node details."""
    node = get_object_or_404(Node, node_id=node_id)
    
    # Get CPU metrics for the last 24 hours
    cpu_metrics = Metric.objects.filter(
        node=node,
        metric_type='node_cpu',
        timestamp__gte=timezone.now() - timedelta(hours=24)
    ).order_by('timestamp').values('timestamp', 'value')
    
    # Get memory metrics for the last 24 hours
    memory_metrics = Metric.objects.filter(
        node=node,
        metric_type='node_memory',
        timestamp__gte=timezone.now() - timedelta(hours=24)
    ).order_by('timestamp').values('timestamp', 'value')
    
    return JsonResponse({
        'node': {
            'id': node.id,
            'node_id': node.node_id,
            'hostname': node.hostname,
            'ip_address': node.ip_address,
            'cpu_cores': node.cpu_cores,
            'memory_mb': node.memory_mb,
            'status': node.status,
            'last_seen': node.last_seen,
            'created_at': node.created_at,
            'updated_at': node.updated_at,
        },
        'cpu_metrics': list(cpu_metrics),
        'memory_metrics': list(memory_metrics),
    })

@login_required
@require_http_methods(["GET"])
def api_vms(request):
    """API endpoint for VMs."""
    vms = VirtualMachine.objects.all().values()
    return JsonResponse(list(vms), safe=False)

@login_required
@require_http_methods(["GET"])
def api_vm_detail(request, vm_id):
    """API endpoint for VM details."""
    vm = get_object_or_404(VirtualMachine, vm_id=vm_id)
    
    # Get CPU metrics for the last 24 hours
    cpu_metrics = Metric.objects.filter(
        vm=vm,
        metric_type='vm_cpu',
        timestamp__gte=timezone.now() - timedelta(hours=24)
    ).order_by('timestamp').values('timestamp', 'value')
    
    # Get memory metrics for the last 24 hours
    memory_metrics = Metric.objects.filter(
        vm=vm,
        metric_type='vm_memory',
        timestamp__gte=timezone.now() - timedelta(hours=24)
    ).order_by('timestamp').values('timestamp', 'value')
    
    return JsonResponse({
        'vm': {
            'id': vm.id,
            'vm_id': vm.vm_id,
            'job_id': vm.job_id,
            'node_id': vm.node.node_id if vm.node else None,
            'buyer_id': vm.buyer_id,
            'vcpu_count': vm.vcpu_count,
            'memory_mb': vm.memory_mb,
            'status': vm.status,
            'created_at': vm.created_at,
            'updated_at': vm.updated_at,
            'terminated_at': vm.terminated_at,
        },
        'cpu_metrics': list(cpu_metrics),
        'memory_metrics': list(memory_metrics),
    })

@login_required
@require_http_methods(["GET"])
def api_marketplace_stats(request):
    """API endpoint for marketplace statistics."""
    # Get active nodes count
    active_nodes_count = Node.objects.filter(status='online').count()
    
    # Get active VMs count
    active_vms_count = VirtualMachine.objects.filter(status='running').count()
    
    # Get total transaction volume
    total_volume = Transaction.objects.filter(
        status='completed',
        transaction_type='payment'
    ).aggregate(total=Sum('amount'))['total'] or 0
    
    # Get marketplace volume metrics for the last 7 days
    volume_metrics = Metric.objects.filter(
        metric_type='marketplace_volume',
        timestamp__gte=timezone.now() - timedelta(days=7)
    ).order_by('timestamp').values('timestamp', 'value')
    
    # Get active nodes metrics for the last 7 days
    nodes_metrics = Metric.objects.filter(
        metric_type='active_nodes',
        timestamp__gte=timezone.now() - timedelta(days=7)
    ).order_by('timestamp').values('timestamp', 'value')
    
    # Get active VMs metrics for the last 7 days
    vms_metrics = Metric.objects.filter(
        metric_type='active_vms',
        timestamp__gte=timezone.now() - timedelta(days=7)
    ).order_by('timestamp').values('timestamp', 'value')
    
    return JsonResponse({
        'active_nodes_count': active_nodes_count,
        'active_vms_count': active_vms_count,
        'total_volume': total_volume,
        'volume_metrics': list(volume_metrics),
        'nodes_metrics': list(nodes_metrics),
        'vms_metrics': list(vms_metrics),
    })

# HTMX views
@login_required
@require_http_methods(["GET"])
def htmx_nodes_table(request):
    """HTMX endpoint for nodes table."""
    nodes_list = Node.objects.all()
    paginator = Paginator(nodes_list, 10)
    page_number = request.GET.get('page', 1)
    nodes = paginator.get_page(page_number)
    
    return render(request, 'marketplace_monitor/partials/nodes_table.html', {
        'nodes': nodes,
    })

@login_required
@require_http_methods(["GET"])
def htmx_vms_table(request):
    """HTMX endpoint for VMs table."""
    vms_list = VirtualMachine.objects.all()
    paginator = Paginator(vms_list, 10)
    page_number = request.GET.get('page', 1)
    vms = paginator.get_page(page_number)
    
    return render(request, 'marketplace_monitor/partials/vms_table.html', {
        'vms': vms,
    })

@login_required
@require_http_methods(["GET"])
def htmx_transactions_table(request):
    """HTMX endpoint for transactions table."""
    transactions_list = Transaction.objects.all()
    paginator = Paginator(transactions_list, 10)
    page_number = request.GET.get('page', 1)
    transactions = paginator.get_page(page_number)
    
    return render(request, 'marketplace_monitor/partials/transactions_table.html', {
        'transactions': transactions,
    })

@login_required
@require_http_methods(["GET"])
def htmx_marketplace_stats(request):
    """HTMX endpoint for marketplace statistics."""
    # Get active nodes count
    active_nodes_count = Node.objects.filter(status='online').count()
    
    # Get active VMs count
    active_vms_count = VirtualMachine.objects.filter(status='running').count()
    
    # Get total transaction volume
    total_volume = Transaction.objects.filter(
        status='completed',
        transaction_type='payment'
    ).aggregate(total=Sum('amount'))['total'] or 0
    
    return render(request, 'marketplace_monitor/partials/marketplace_stats_cards.html', {
        'active_nodes_count': active_nodes_count,
        'active_vms_count': active_vms_count,
        'total_volume': total_volume,
    })

# WebSocket handler
@login_required
@require_http_methods(["POST"])
def ws_connect(request):
    """Connect to the WebSocket server."""
    asyncio.run(ws_client.connect())
    return JsonResponse({"status": "connected"})

@login_required
@require_http_methods(["POST"])
def ws_disconnect(request):
    """Disconnect from the WebSocket server."""
    asyncio.run(ws_client.disconnect())
    return JsonResponse({"status": "disconnected"})
