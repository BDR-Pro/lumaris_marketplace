// File: marketplace/node_ws/src/http_client.rs

use crate::error::{Result, NodeError};
use log::{info, error, debug};
use serde_json::json;

const ADMIN_API_URL: &str = "http://127.0.0.1:8000";

/// Update node availability in the admin API
pub async fn update_node_availability(
    node_id: &str,
    cpu_available: f64,
    mem_available: u32,
    status: &str
) -> Result<()> {
    let url = format!("{}/nodes/update", ADMIN_API_URL);
    
    debug!("Updating node availability for {}: CPU={}, MEM={}, status={}", 
        node_id, cpu_available, mem_available, status);
    
    // Create the payload
    let payload = json!({
        "node_id": node_id,
        "cpu_available": cpu_available,
        "mem_available": mem_available,
        "status": status,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Send the request
    match ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(payload)
    {
        Ok(_) => {
            debug!("Successfully updated node availability for {}", node_id);
            Ok(())
        },
        Err(e) => {
            error!("Failed to update node availability: {}", e);
            Err(NodeError::HttpClientError(format!("Failed to update node availability: {}", e)))
        }
    }
}

/// Get job details from the admin API
pub async fn get_job_details(job_id: u64) -> Result<serde_json::Value> {
    let url = format!("{}/jobs/{}", ADMIN_API_URL, job_id);
    
    debug!("Getting job details for job ID: {}", job_id);
    
    // Send the request
    match ureq::get(&url).call() {
        Ok(response) => {
            let json: serde_json::Value = response.into_json()
                .map_err(|e| NodeError::SerializationError(e))?;
            
            debug!("Successfully retrieved job details for job ID: {}", job_id);
            Ok(json)
        },
        Err(e) => {
            error!("Failed to get job details: {}", e);
            Err(NodeError::HttpClientError(format!("Failed to get job details: {}", e)))
        }
    }
}

/// Update job status in the admin API
pub async fn update_job_status(
    job_id: u64,
    status: &str,
    output: Option<&str>,
    error: Option<&str>
) -> Result<()> {
    let url = format!("{}/jobs/{}/update", ADMIN_API_URL, job_id);
    
    debug!("Updating job status for job ID {}: status={}", job_id, status);
    
    // Create the payload
    let mut payload = json!({
        "status": status,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Add output and error if provided
    if let Some(output_str) = output {
        payload["output"] = json!(output_str);
    }
    
    if let Some(error_str) = error {
        payload["error"] = json!(error_str);
    }
    
    // Send the request
    match ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(payload)
    {
        Ok(_) => {
            debug!("Successfully updated job status for job ID: {}", job_id);
            Ok(())
        },
        Err(e) => {
            error!("Failed to update job status: {}", e);
            Err(NodeError::HttpClientError(format!("Failed to update job status: {}", e)))
        }
    }
}

