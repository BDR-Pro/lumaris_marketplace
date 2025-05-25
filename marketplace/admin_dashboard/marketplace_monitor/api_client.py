"""
API client for communicating with the FastAPI backend.
"""
import json
import logging
import httpx
from django.conf import settings
from datetime import datetime
from typing import Dict, List, Any, Optional, Union

logger = logging.getLogger(__name__)

class FastAPIClient:
    """Client for interacting with the FastAPI backend."""
    
    def __init__(self):
        self.base_url = settings.FASTAPI_URL
        self.api_key = settings.API_KEY
        self.headers = {
            "Content-Type": "application/json",
            "X-API-Key": self.api_key
        }
    
    async def _make_request(self, method: str, endpoint: str, data: Optional[Dict] = None) -> Dict:
        """Make an HTTP request to the FastAPI backend."""
        url = f"{self.base_url}{endpoint}"
        
        try:
            async with httpx.AsyncClient() as client:
                if method.lower() == "get":
                    response = await client.get(url, headers=self.headers, timeout=10.0)
                elif method.lower() == "post":
                    response = await client.post(url, headers=self.headers, json=data, timeout=10.0)
                elif method.lower() == "put":
                    response = await client.put(url, headers=self.headers, json=data, timeout=10.0)
                elif method.lower() == "delete":
                    response = await client.delete(url, headers=self.headers, timeout=10.0)
                else:
                    raise ValueError(f"Unsupported HTTP method: {method}")
                
                response.raise_for_status()
                return response.json()
        except httpx.HTTPStatusError as e:
            logger.error(f"HTTP error: {e.response.status_code} - {e.response.text}")
            try:
                error_data = e.response.json()
                return {"error": error_data.get("detail", str(e))}
            except json.JSONDecodeError:
                return {"error": str(e)}
        except httpx.RequestError as e:
            logger.error(f"Request error: {str(e)}")
            return {"error": str(e)}
        except Exception as e:
            logger.error(f"Unexpected error: {str(e)}")
            return {"error": str(e)}
    
    # Node operations
    async def get_nodes(self) -> List[Dict]:
        """Get all nodes."""
        response = await self._make_request("get", "/nodes/")
        if "error" in response:
            return []
        return response
    
    async def get_node(self, node_id: str) -> Dict:
        """Get a specific node."""
        response = await self._make_request("get", f"/nodes/{node_id}")
        return response
    
    async def register_node(self, node_data: Dict) -> Dict:
        """Register a new node."""
        response = await self._make_request("post", "/nodes/", data=node_data)
        return response
    
    async def update_node(self, node_id: str, node_data: Dict) -> Dict:
        """Update a node."""
        response = await self._make_request("put", f"/nodes/{node_id}", data=node_data)
        return response
    
    async def delete_node(self, node_id: str) -> Dict:
        """Delete a node."""
        response = await self._make_request("delete", f"/nodes/{node_id}")
        return response
    
    # VM operations
    async def get_vms(self) -> List[Dict]:
        """Get all VMs."""
        response = await self._make_request("get", "/vms/")
        if "error" in response:
            return []
        return response
    
    async def get_vm(self, vm_id: str) -> Dict:
        """Get a specific VM."""
        response = await self._make_request("get", f"/vms/{vm_id}")
        return response
    
    async def create_vm(self, vm_data: Dict) -> Dict:
        """Create a new VM."""
        response = await self._make_request("post", "/vms/create", data=vm_data)
        return response
    
    async def terminate_vm(self, vm_id: str) -> Dict:
        """Terminate a VM."""
        response = await self._make_request("post", f"/vms/{vm_id}/terminate")
        return response
    
    # Job operations
    async def get_jobs(self) -> List[Dict]:
        """Get all jobs."""
        response = await self._make_request("get", "/jobs/")
        if "error" in response:
            return []
        return response
    
    async def get_job(self, job_id: int) -> Dict:
        """Get a specific job."""
        response = await self._make_request("get", f"/jobs/{job_id}")
        return response
    
    async def create_job(self, job_data: Dict) -> Dict:
        """Create a new job."""
        response = await self._make_request("post", "/jobs/", data=job_data)
        return response
    
    # Matchmaking operations
    async def update_node_availability(self, node_data: Dict) -> Dict:
        """Update node availability."""
        response = await self._make_request("post", "/matchmaking/node/update_availability", data=node_data)
        return response
    
    async def assign_job(self, assignment_data: Dict) -> Dict:
        """Assign a job to a node."""
        response = await self._make_request("post", "/matchmaking/job/assign", data=assignment_data)
        return response
    
    async def update_job_status(self, status_data: Dict) -> Dict:
        """Update job status."""
        response = await self._make_request("post", "/matchmaking/job/status", data=status_data)
        return response
    
    # Statistics operations
    async def get_marketplace_stats(self) -> Dict:
        """Get marketplace statistics."""
        response = await self._make_request("get", "/stats/marketplace")
        return response
    
    async def get_buyer_stats(self, buyer_id: str) -> Dict:
        """Get buyer statistics."""
        response = await self._make_request("get", f"/stats/buyer/{buyer_id}")
        return response
    
    async def get_seller_stats(self, seller_id: str) -> Dict:
        """Get seller statistics."""
        response = await self._make_request("get", f"/stats/seller/{seller_id}")
        return response
    
    # Health check
    async def health_check(self) -> bool:
        """Check if the API is healthy."""
        try:
            response = await self._make_request("get", "/health")
            return response.get("status") == "healthy"
        except Exception:
            return False


# Create a singleton instance
api_client = FastAPIClient()

