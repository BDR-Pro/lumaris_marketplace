//# node_ws/src/main.rs
mod ws_handler;
mod auth;
mod vm_manager;
mod matchmaker;
mod job_scheduler;
mod http_client;
mod error;
mod config;

use ws_handler::start_ws_server;
use matchmaker::create_matchmaker;
use job_scheduler::create_job_scheduler;
use log::{info, error, debug, warn};
use std::env;
use error::Result;
use config::Config;

#[tokio::main]
async fn main() {
    // Load configuration
    let config_path = env::var("LUMARIS_CONFIG")
        .unwrap_or_else(|_| "../config.toml".to_string());
    
    let config = Config::load(config_path).unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}. Using default configuration.", e);
        Config::default()
    });
    
    // Initialize the logger
    let log_level = match config.logging.level.to_lowercase().as_str() {
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    };
    
    let mut builder = env_logger::Builder::new();
    builder.filter_level(log_level);
    
    if config.logging.console {
        builder.init();
    }
    
    info!("🚀 Starting Lumaris Marketplace Service");
    info!("Configuration loaded from: {}", config_path);
    
    // Create the matchmaker
    let (matchmaker, _matchmaker_rx) = create_matchmaker();
    info!("✅ Matchmaker initialized");
    
    // Create the job scheduler
    let scheduler = create_job_scheduler(matchmaker.clone());
    info!("✅ Job scheduler initialized");
    
    // Start REST API for job submission
    let rest_host = config.rest_api.host.clone();
    let rest_port = config.rest_api.port;
    
    tokio::spawn(async move {
        info!("🔄 Starting REST API server on {}:{} (HTTP)...", rest_host, rest_port);
        if let Err(e) = start_rest_api(matchmaker.clone(), scheduler.clone()).await {
            error!("REST API server error: {}", e);
        }
    });
    
    // Start WebSocket server for node connections
    let ws_host = config.node_ws.host.clone();
    let ws_port = config.node_ws.port;
    
    info!("🔄 Starting WebSocket Server on {}:{} (WS)...", ws_host, ws_port);
    if let Err(e) = start_ws_server(&ws_host, ws_port, matchmaker.clone()).await {
        error!("WebSocket server error: {}", e);
    }
}

// REST API for job submission
async fn start_rest_api(
    matchmaker: std::sync::Arc<std::sync::Mutex<matchmaker::MatchMaker>>,
    scheduler: std::sync::Arc<std::sync::Mutex<job_scheduler::JobScheduler>>,
) -> Result<()> {
    use warp::{Filter, Reply, Rejection, http::Response};
    use serde_json::{json, Value};
    
    // Submit job endpoint
    let submit_job = warp::path!("jobs")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |job_data: Value| {
            let matchmaker_clone = matchmaker.clone();
            let scheduler_clone = scheduler.clone();
            
            async move {
                let result = handle_job_submission(job_data, &matchmaker_clone, &scheduler_clone).await;
                match result {
                    Ok(job_id) => {
                        let response = json!({
                            "status": "success",
                            "job_id": job_id,
                            "message": "Job submitted successfully"
                        });
                        
                        Ok::<_, Rejection>(Response::builder()
                            .status(200)
                            .header("Content-Type", "application/json")
                            .body(response.to_string())
                            .unwrap())
                    },
                    Err(e) => {
                        let response = json!({
                            "status": "error",
                            "message": format!("Failed to submit job: {}", e)
                        });
                        
                        Ok::<_, Rejection>(Response::builder()
                            .status(400)
                            .header("Content-Type", "application/json")
                            .body(response.to_string())
                            .unwrap())
                    }
                }
            }
        });
    
    // Get job status endpoint
    let get_job = warp::path!("jobs" / u64)
        .and(warp::get())
        .and_then(move |job_id: u64| {
            let scheduler_clone = scheduler.clone();
            
            async move {
                let result = handle_get_job(job_id, &scheduler_clone).await;
                match result {
                    Ok(job_data) => {
                        Ok::<_, Rejection>(Response::builder()
                            .status(200)
                            .header("Content-Type", "application/json")
                            .body(job_data.to_string())
                            .unwrap())
                    },
                    Err(e) => {
                        let response = json!({
                            "status": "error",
                            "message": format!("Failed to get job: {}", e)
                        });
                        
                        Ok::<_, Rejection>(Response::builder()
                            .status(404)
                            .header("Content-Type", "application/json")
                            .body(response.to_string())
                            .unwrap())
                    }
                }
            }
        });
    
    // Health check endpoint
    let healthcheck = warp::path!("health")
        .map(|| {
            Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(json!({
                    "status": "ok",
                    "version": env!("CARGO_PKG_VERSION"),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }).to_string())
                .unwrap()
        });
    
    // Combine all routes
    let routes = submit_job
        .or(get_job)
        .or(healthcheck)
        .with(warp::cors().allow_any_origin());
    
    // Start the server
    warp::serve(routes)
        .run(([127, 0, 0, 1], 9002))
        .await;
    
    Ok(())
}

// Handle job submission
async fn handle_job_submission(
    job_data: serde_json::Value,
    matchmaker: &std::sync::Arc<std::sync::Mutex<matchmaker::MatchMaker>>,
    scheduler: &std::sync::Arc<std::sync::Mutex<job_scheduler::JobScheduler>>,
) -> Result<u64> {
    // In a real implementation, we would:
    // 1. Validate the job data
    // 2. Create a job in the scheduler
    // 3. Find a suitable node using the matchmaker
    // 4. Dispatch the job to the node
    
    // For now, just return a dummy job ID
    Ok(12345)
}

// Handle get job request
async fn handle_get_job(
    job_id: u64,
    scheduler: &std::sync::Arc<std::sync::Mutex<job_scheduler::JobScheduler>>,
) -> Result<serde_json::Value> {
    // In a real implementation, we would:
    // 1. Get the job from the scheduler
    // 2. Return the job data
    
    // For now, just return dummy data
    Ok(serde_json::json!({
        "job_id": job_id,
        "status": "running",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "updated_at": chrono::Utc::now().to_rfc3339(),
    }))
}
