use serde_json::{json, Value};
use log::{info, error};

// Update node availability in the API
pub async fn update_node_availability(
    api_url: &str,
    node_id: &str,
    available: bool
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    
    let payload = json!({
        "node_id": node_id,
        "available": available
    });
    
    let response = client.post(&format!("{}/api/nodes/{}/availability", api_url, node_id))
        .json(&payload)
        .send()
        .await?;
    
    if response.status().is_success() {
        info!("Node {} availability updated to {} in API", node_id, available);
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await?;
        error!("Failed to update node availability: HTTP {}: {}", status, error_text);
        Err(format!("API error: {}", status).into())
    }
}

// Update job status in the API
pub async fn update_job_status(
    api_url: &str,
    job_id: u64,
    status: &str,
    result_data: Option<Value>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    
    let mut payload = json!({
        "job_id": job_id,
        "status": status
    });
    
    if let Some(result) = result_data {
        payload["result"] = result;
    }
    
    let response = client.post(&format!("{}/api/jobs/{}/status", api_url, job_id))
        .json(&payload)
        .send()
        .await?;
    
    if response.status().is_success() {
        info!("Job {} status updated to {} in API", job_id, status);
        Ok(())
    } else {
        let status = response.status();
        let error_text = response.text().await?;
        error!("Failed to update job status: HTTP {}: {}", status, error_text);
        Err(format!("API error: {}", status).into())
    }
}

