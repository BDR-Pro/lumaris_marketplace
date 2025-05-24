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
    
    let config = Config::load(&config_path).unwrap_or_else(|e| {
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
    let matchmaker_for_rest = matchmaker.clone();
    let scheduler_for_rest = scheduler.clone();
    
    tokio::spawn(async move {
        info!("🔄 Starting REST API server on {}:{} (HTTP)...", rest_host, rest_port);
        if let Err(e) = start_rest_api(matchmaker_for_rest, scheduler_for_rest).await {
            error!("REST API server error: {}", e);
        }
    });
    
    // Start WebSocket server for node connections
    let ws_host = config.node_ws.host.clone();
    let ws_port = config.node_ws.port;
    let matchmaker_for_ws = matchmaker.clone();
    
    info!("🔄 Starting WebSocket Server on {}:{} (WS)...", ws_host, ws_port);
    if let Err(e) = start_ws_server(&ws_host, ws_port, matchmaker_for_ws).await {
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
        .and_then({
            let matchmaker = matchmaker.clone();
            let scheduler = scheduler.clone();
            move |job_data: Value| {
                let matchmaker_clone = matchmaker.clone();
                let scheduler_clone = scheduler.clone();
                
                async move {
                    let result = handle_job_submission(job_data, &matchmaker_clone, &scheduler_clone).await;
                    match result {
                        Ok(job_id) => {
                            let response = warp::reply::json(&json!({
                                "status": "success",
                                "job_id": job_id,
                                "message": "Job submitted successfully"
                            }));
                            Ok::<_, Rejection>(response)
                        },
                        Err(e) => {
                            let response = warp::reply::json(&json!({
                                "status": "error",
                                "message": format!("Failed to submit job: {}", e)
                            }));
                            Ok::<_, Rejection>(response)
                        }
                    }
                }
            }
        });
    
    // Get job status endpoint
    let get_job_status = warp::path!("jobs" / u64 / "status")
        .and(warp::get())
        .and_then({
            let matchmaker = matchmaker.clone();
            let scheduler = scheduler.clone();
            move |job_id: u64| {
                let matchmaker_clone = matchmaker.clone();
                let scheduler_clone = scheduler.clone();
                
                async move {
                    let result = handle_job_status_request(job_id, &matchmaker_clone, &scheduler_clone).await;
                    match result {
                        Ok(status) => {
                            let response = warp::reply::json(&status);
                            Ok::<_, Rejection>(response)
                        },
                        Err(e) => {
                            let response = warp::reply::json(&json!({
                                "status": "error",
                                "message": format!("Failed to get job status: {}", e)
                            }));
                            Ok::<_, Rejection>(response)
                        }
                    }
                }
            }
        });
    
    // Combine routes
    let routes = submit_job
        .or(get_job_status)
        .with(warp::cors().allow_any_origin());
    
    // Start the server
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8000))
        .await;
    
    Ok(())
}

// Handle job submission
async fn handle_job_submission(
    job_data: serde_json::Value,
    matchmaker: &std::sync::Arc<std::sync::Mutex<matchmaker::MatchMaker>>,
    _scheduler: &std::sync::Arc<std::sync::Mutex<job_scheduler::JobScheduler>>,
) -> Result<u64> {
    // Extract job requirements
    let cpu_cores = job_data["requirements"]["cpu_cores"].as_f64().unwrap_or(1.0) as f32;
    let memory_mb = job_data["requirements"]["memory_mb"].as_u64().unwrap_or(1024);
    let expected_duration = job_data["requirements"]["expected_duration_sec"].as_u64().unwrap_or(3600);
    let priority = job_data["requirements"]["priority"].as_u64().unwrap_or(1) as u8;
    
    // Create job requirements
    let job_reqs = matchmaker::JobRequirements {
        job_id: 0, // Will be assigned by matchmaker
        cpu_cores,
        memory_mb,
        expected_duration_sec: expected_duration,
        priority,
    };
    
    // Create the job
    let job = matchmaker::Job {
        id: 0, // Will be assigned by matchmaker
        requirements: job_reqs,
        status: matchmaker::JobStatus::Queued,
        assigned_node: None,
        submitted_at: chrono::Utc::now().timestamp() as u64,
        started_at: None,
        completed_at: None,
    };
    
    // Submit to matchmaker
    let job_id = {
        let mut mm = matchmaker.lock().unwrap();
        mm.submit_job(job)
    };
    
    Ok(job_id)
}

// Handle job status request
async fn handle_job_status_request(
    job_id: u64,
    matchmaker: &std::sync::Arc<std::sync::Mutex<matchmaker::MatchMaker>>,
    _scheduler: &std::sync::Arc<std::sync::Mutex<job_scheduler::JobScheduler>>,
) -> Result<serde_json::Value> {
    // Get job status from matchmaker
    let status = {
        let mm = matchmaker.lock().unwrap();
        // In a real implementation, you would have a method to get job status
        // For now, we'll just return a placeholder
        serde_json::json!({
            "job_id": job_id,
            "status": "queued",
            "timestamp": chrono::Utc::now().timestamp()
        })
    };
    
    Ok(status)
}
