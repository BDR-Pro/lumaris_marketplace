use log::{error, debug};
use serde_json::{json, Value};
use crate::error::{NodeError, Result};

pub async fn update_node_status(
    api_url: &str,
    node_id: &str,
    status: &str
) -> Result<()> {
    let url = format!("{}/api/nodes/{}/status", api_url, node_id);
    
    // Create the payload
    let payload = json!({
        "status": status,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Send the request
    match reqwest::Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .await {
            Ok(_) => {
                debug!("Updated node status: {}", status);
                Ok(())
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
) -> Result<()> {
    let url = format!("{}/api/nodes/{}/availability", api_url, node_id);
    
    // Create the payload
    let payload = json!({
        "available": available,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Send the request
    match reqwest::Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .await {
            Ok(_) => {
                debug!("Updated node availability: {}", available);
                Ok(())
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
    node_id: &str
) -> Result<()> {
    let url = format!("{}/api/jobs/{}/assign", api_url, job_id);
    
    // Create the payload
    let payload = json!({
        "node_id": node_id,
        "timestamp": chrono::Utc::now().timestamp()
    });
    
    // Send the request
    match reqwest::Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .await {
            Ok(_) => {
                debug!("Assigned job {} to node {}", job_id, node_id);
                Ok(())
            },
            Err(e) => {
                error!("Failed to assign job: {}", e);
                Err(NodeError::ApiError(format!("Failed to assign job: {}", e)))
            }
        }
}

pub async fn update_job_status(
    api_url: &str,
    job_id: u64,
    status: &str,
    result_data: Option<Value>
) -> Result<()> {
    let url = format!("{}/api/jobs/{}/status", api_url, job_id);
    
    // Create the payload based on whether result data is available
    let payload = match result_data {
        Some(data) => json!({
            "status": status,
            "timestamp": chrono::Utc::now().timestamp(),
            "result": data
        }),
        None => json!({
            "status": status,
            "timestamp": chrono::Utc::now().timestamp()
        })
    };
    
    // Send the request
    match reqwest::Client::new()
        .post(&url)
        .json(&payload)
        .send()
        .await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    debug!("Updated job status: {}", status);
                    Ok(())
                } else {
                    let error_msg = format!("API returned error status: {}", status);
                    error!("{}", error_msg);
                    Err(NodeError::ApiError(error_msg))
                }
            },
            Err(e) => {
                error!("Failed to update job status: {}", e);
                Err(NodeError::ApiError(format!("Failed to update job status: {}", e)))
            }
        }
}
