from django.urls import path
from . import views

app_name = 'marketplace_monitor'

urlpatterns = [
    # Dashboard views
    path('', views.dashboard, name='dashboard'),
    path('nodes/', views.nodes, name='nodes'),
    path('nodes/<str:node_id>/', views.node_detail, name='node_detail'),
    path('vms/', views.vms, name='vms'),
    path('vms/<str:vm_id>/', views.vm_detail, name='vm_detail'),
    path('transactions/', views.transactions, name='transactions'),
    path('stats/', views.marketplace_stats, name='marketplace_stats'),
    
    # API views
    path('api/nodes/', views.api_nodes, name='api_nodes'),
    path('api/nodes/<str:node_id>/', views.api_node_detail, name='api_node_detail'),
    path('api/vms/', views.api_vms, name='api_vms'),
    path('api/vms/<str:vm_id>/', views.api_vm_detail, name='api_vm_detail'),
    path('api/stats/', views.api_marketplace_stats, name='api_marketplace_stats'),
    
    # HTMX views
    path('htmx/nodes-table/', views.htmx_nodes_table, name='htmx_nodes_table'),
    path('htmx/vms-table/', views.htmx_vms_table, name='htmx_vms_table'),
    path('htmx/transactions-table/', views.htmx_transactions_table, name='htmx_transactions_table'),
    path('htmx/marketplace-stats/', views.htmx_marketplace_stats, name='htmx_marketplace_stats'),
    
    # WebSocket handler
    path('ws/connect/', views.ws_connect, name='ws_connect'),
    path('ws/disconnect/', views.ws_disconnect, name='ws_disconnect'),
]

