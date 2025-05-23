use serde::Serialize;
use reqwest::Client;

const API_BASE: &str = "http://localhost:8000/matchmaking"; // change if deployed
const JWT_TOKEN: &str = "super-secret"; // match your Python backend

#[derive(Serialize)]
struct NodeAvailabilityUpdate {
    node_id: String,
    cpu_available: f64,
    mem_available: u32,
    status: String,
}

#[derive(Serialize)]
struct JobAssignment {
    job_id: String,
    node_id: String,
}

#[derive(Serialize)]
struct JobStatusUpdate {
    job_id: String,
    status: String,
    progress: Option<f32>,
    cpu_time_sec: Option<u32>,
    peak_memory_mb: Option<u32>,
}

pub async fn update_node_availability(
    node_id: &str,
    cpu: f64,
    mem: u32,
    status: &str,
) -> Result<(), reqwest::Error> {
    let payload = NodeAvailabilityUpdate {
        node_id: node_id.to_string(),
        cpu_available: cpu,
        mem_available: mem,
        status: status.to_string(),
    };

    let client = Client::new();
    let res = client
        .post(&format!("{}/node/update_availability", API_BASE))
        .header("Authorization", format!("Bearer {}", JWT_TOKEN))
        .json(&payload)
        .send()
        .await?;

    println!("Update node response: {:?}", res.status());
    Ok(())
}

pub async fn assign_job(job_id: &str, node_id: &str) -> Result<(), reqwest::Error> {
    let payload = JobAssignment {
        job_id: job_id.to_string(),
        node_id: node_id.to_string(),
    };

    let client = Client::new();
    let res = client
        .post(&format!("{}/job/assign", API_BASE))
        .header("Authorization", format!("Bearer {}", JWT_TOKEN))
        .json(&payload)
        .send()
        .await?;

    println!("Assign job response: {:?}", res.status());
    Ok(())
}

pub async fn update_job_status(
    job_id: &str,
    status: &str,
    progress: Option<f32>,
    cpu_time: Option<u32>,
    mem_peak: Option<u32>,
) -> Result<(), reqwest::Error> {
    let payload = JobStatusUpdate {
        job_id: job_id.to_string(),
        status: status.to_string(),
        progress,
        cpu_time_sec: cpu_time,
        peak_memory_mb: mem_peak,
    };

    let client = Client::new();
    let res = client
        .post(&format!("{}/job/status", API_BASE))
        .header("Authorization", format!("Bearer {}", JWT_TOKEN))
        .json(&payload)
        .send()
        .await?;

    println!("Job status update: {:?}", res.status());
    Ok(())
}
