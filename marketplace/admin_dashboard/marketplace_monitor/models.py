from django.db import models
from django.utils import timezone

class Node(models.Model):
    """Model representing a seller node in the marketplace."""
    node_id = models.CharField(max_length=255, unique=True)
    hostname = models.CharField(max_length=255)
    ip_address = models.GenericIPAddressField(null=True, blank=True)
    cpu_cores = models.FloatField(default=0.0)
    memory_mb = models.IntegerField(default=0)
    status = models.CharField(max_length=50, default='offline')
    last_seen = models.DateTimeField(default=timezone.now)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    
    def __str__(self):
        return f"{self.node_id} ({self.status})"
    
    class Meta:
        ordering = ['-last_seen']

class VirtualMachine(models.Model):
    """Model representing a virtual machine in the marketplace."""
    vm_id = models.CharField(max_length=255, unique=True)
    job_id = models.IntegerField()
    node = models.ForeignKey(Node, on_delete=models.SET_NULL, null=True, related_name='vms')
    buyer_id = models.CharField(max_length=255)
    vcpu_count = models.IntegerField(default=1)
    memory_mb = models.IntegerField(default=1024)
    status = models.CharField(max_length=50, default='creating')
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    terminated_at = models.DateTimeField(null=True, blank=True)
    
    def __str__(self):
        return f"VM {self.vm_id} ({self.status})"
    
    class Meta:
        ordering = ['-created_at']

class Transaction(models.Model):
    """Model representing a financial transaction in the marketplace."""
    TRANSACTION_TYPES = (
        ('payment', 'Payment'),
        ('refund', 'Refund'),
        ('deposit', 'Deposit'),
        ('withdrawal', 'Withdrawal'),
    )
    
    transaction_id = models.CharField(max_length=255, unique=True)
    user_id = models.CharField(max_length=255)
    user_type = models.CharField(max_length=50)  # buyer or seller
    amount = models.DecimalField(max_digits=10, decimal_places=2)
    transaction_type = models.CharField(max_length=50, choices=TRANSACTION_TYPES)
    vm = models.ForeignKey(VirtualMachine, on_delete=models.SET_NULL, null=True, blank=True, related_name='transactions')
    status = models.CharField(max_length=50, default='pending')
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    
    def __str__(self):
        return f"{self.transaction_type} of {self.amount} for {self.user_id}"
    
    class Meta:
        ordering = ['-created_at']

class Metric(models.Model):
    """Model for storing time-series metrics data."""
    METRIC_TYPES = (
        ('node_cpu', 'Node CPU Usage'),
        ('node_memory', 'Node Memory Usage'),
        ('vm_cpu', 'VM CPU Usage'),
        ('vm_memory', 'VM Memory Usage'),
        ('marketplace_volume', 'Marketplace Volume'),
        ('active_nodes', 'Active Nodes'),
        ('active_vms', 'Active VMs'),
    )
    
    metric_type = models.CharField(max_length=50, choices=METRIC_TYPES)
    value = models.FloatField()
    node = models.ForeignKey(Node, on_delete=models.CASCADE, null=True, blank=True, related_name='metrics')
    vm = models.ForeignKey(VirtualMachine, on_delete=models.CASCADE, null=True, blank=True, related_name='metrics')
    timestamp = models.DateTimeField(default=timezone.now)
    
    def __str__(self):
        return f"{self.metric_type}: {self.value} at {self.timestamp}"
    
    class Meta:
        ordering = ['-timestamp']
        indexes = [
            models.Index(fields=['metric_type', 'timestamp']),
            models.Index(fields=['node', 'metric_type', 'timestamp']),
            models.Index(fields=['vm', 'metric_type', 'timestamp']),
        ]
