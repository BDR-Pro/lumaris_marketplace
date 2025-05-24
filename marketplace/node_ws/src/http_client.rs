// File: marketplace/node_ws/src/http_client.rs

use crate::error::{Result, NodeError};
use log::{info, error, debug};
use serde_json::{json, Value};
use std::collections::HashMap;

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

/// Assign a job to a node
pub async fn assign_job(
    node_id: &str, 
    job_id: u64, 
    chunk_id: u64, 
    payload: &HashMap<String, Value>
) -> Result<bool> {
    let url = format!("http://127.0.0.1:8000/nodes/{}/jobs", node_id);
    
    let job_data = json!({
        "job_id": job_id,
        "chunk_id": chunk_id,
        "payload": payload
    });
    
    debug!("Assigning job {} to node {}", job_id, node_id);
    
    // In a real implementation, this would be an async HTTP request
    // For now, we'll use a blocking request in a separate thread
    let job_json = job_data.to_string();
    
    // Spawn a blocking task for the HTTP request
    let result = tokio::task::spawn_blocking(move || {
        match ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_string(&job_json)
        {
            Ok(_) => Ok(true),
            Err(e) => Err(NodeError::ApiError(format!("Failed to assign job: {}", e)))
        }
    }).await;
    
    match result {
        Ok(inner_result) => inner_result,
        Err(e) => Err(NodeError::ApiError(format!("Task join error: {}", e)))
    }
}

/// Update job status in the admin API
pub async fn update_job_status(
    job_id: u64, 
    status: &str, 
    result: Option<Value>
) -> Result<Value> {
    let url = format!("http://127.0.0.1:8000/jobs/{}/status", job_id);
    
    let status_data = json!({
        "status": status,
        "result": result
    });
    
    debug!("Updating job {} status to {}", job_id, status);
    
    // In a real implementation, this would be an async HTTP request
    let status_json = status_data.to_string();
    
    // Spawn a blocking task for the HTTP request
    let result = tokio::task::spawn_blocking(move || {
        match ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_string(&status_json)
        {
            Ok(response) => {
                match response.into_json::<Value>() {
                    Ok(json) => Ok(json),
                    Err(e) => Err(NodeError::ApiError(format!("Failed to parse response: {}", e)))
                }
            },
            Err(e) => Err(NodeError::ApiError(format!("Failed to update job status: {}", e)))
        }
    }).await;
    
    match result {
        Ok(inner_result) => inner_result,
        Err(e) => Err(NodeError::ApiError(format!("Task join error: {}", e)))
    }
}
