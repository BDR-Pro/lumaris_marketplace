use log::{info, error, debug};
use serde_json::{json, Value};
use tokio::task;
use crate::error::{NodeError, Result};

pub async fn update_node_status(
    api_url: &str, 
    node_id: &str, 
    cpu_usage: f32, 
    memory_usage: f32, 
    available: bool
) -> Result<bool> {
    let url = format!("{}/nodes/{}/status", api_url, node_id);
    
    // Create the payload
    let payload = json!({
        "cpu_usage": cpu_usage,
        "memory_usage": memory_usage,
        "available": available,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    debug!("Updating node status: {}", payload);
    
    let url = url.clone();
    let payload_str = payload.to_string();
    
    // Send the request
    match ureq::post(&url)
        .content_type("application/json")
        .send(payload_str)
    {
        Ok(_) => {
            debug!("Node status updated successfully");
            Ok(true)
        },
        Err(e) => {
            error!("Failed to update node status: {}", e);
            Err(NodeError::ApiError(format!("Failed to update node status: {}", e)))
        }
    }
}

pub async fn update_node_availability(
    api_url: &str,
    node_id: &str,
    available: bool
) -> Result<bool> {
    let url = format!("{}/nodes/{}/availability", api_url, node_id);
    
    // Create the payload
    let payload = json!({
        "available": available,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    let payload_str = payload.to_string();
    
    // Send the request
    match ureq::post(&url)
        .content_type("application/json")
        .send(payload_str)
    {
        Ok(_) => {
            debug!("Node availability updated successfully");
            Ok(true)
        },
        Err(e) => {
            error!("Failed to update node availability: {}", e);
            Err(NodeError::ApiError(format!("Failed to update node availability: {}", e)))
        }
    }
}

pub async fn assign_job(
    api_url: &str,
    job_id: u64,
    node_id: &str,
    job_data: Value
) -> Result<bool> {
    let url = format!("{}/jobs/{}/assign/{}", api_url, job_id, node_id);
    
    // Create the job assignment payload
    let job_json = serde_json::to_string(&job_data).map_err(|e| {
        NodeError::SerializationError(e)
    })?;
    
    let url = url.clone();
    
    // Spawn a blocking task for the HTTP request
    let result = tokio::task::spawn_blocking(move || {
        match ureq::post(&url)
            .content_type("application/json")
            .send(job_json)
        {
            Ok(_) => Ok(true),
            Err(e) => Err(NodeError::ApiError(format!("Failed to assign job: {}", e)))
        }
    }).await;
    
    match result {
        Ok(res) => res,
        Err(e) => Err(NodeError::ApiError(format!("Task join error: {}", e)))
    }
}

pub async fn update_job_status(
    api_url: &str,
    job_id: u64,
    status: &str,
    result_data: Option<Value>
) -> Result<Value> {
    let url = format!("{}/jobs/{}/status", api_url, job_id);
    
    // Create the status update payload
    let status_payload = match result_data {
        Some(data) => json!({
            "status": status,
            "result": data,
            "timestamp": chrono::Utc::now().timestamp()
        }),
        None => json!({
            "status": status,
            "timestamp": chrono::Utc::now().timestamp()
        })
    };
    
    let status_json = serde_json::to_string(&status_payload).map_err(|e| {
        NodeError::SerializationError(e)
    })?;
    
    let url = url.clone();
    
    // Spawn a blocking task for the HTTP request
    let result = tokio::task::spawn_blocking(move || {
        match ureq::post(&url)
            .content_type("application/json")
            .send(status_json)
        {
            Ok(response) => {
                // Read the response body as a string
                let body = response.into_string()?;
                
                // Parse the string as JSON
                match serde_json::from_str::<Value>(&body) {
                    Ok(json) => Ok(json),
                    Err(e) => Err(NodeError::SerializationError(e))
                }
            },
            Err(e) => Err(NodeError::ApiError(format!("Failed to update job status: {}", e)))
        }
    }).await;
    
    match result {
        Ok(res) => res,
        Err(e) => Err(NodeError::ApiError(format!("Task join error: {}", e)))
    }
}
