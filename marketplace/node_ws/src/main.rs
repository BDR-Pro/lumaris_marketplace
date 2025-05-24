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
    
    info!("\ud83d\ude80 Starting Lumaris Marketplace Service");
    info!("Configuration loaded from: {}", config_path);
    
    // Create the matchmaker
    let (matchmaker, _matchmaker_rx) = create_matchmaker();
    info!("\u2705 Matchmaker initialized");
    
    // Create the job scheduler
    let scheduler = create_job_scheduler(matchmaker.clone());
    info!("\u2705 Job scheduler initialized");
    
    // Start REST API for job submission
    let rest_host = config.rest_api.host.clone();
    let rest_port = config.rest_api.port;
    
    tokio::spawn(async move {
        info!("\ud83d\udd04 Starting REST API server on http://{}:{}...", rest_host, rest_port);
        if let Err(e) = start_rest_api(matchmaker.clone(), scheduler.clone()).await {
            error!("REST API server error: {}", e);
        }
    });
    
    // Start WebSocket server for node connections
    let ws_host = config.node_ws.host.clone();
    let ws_port = config.node_ws.port;
    
    info!("\ud83d\udd04 Starting WebSocket Server on ws://{}:{}...", ws_host, ws_port);
    if let Err(e) = start_ws_server().await {
        error!("WebSocket server error: {}", e);
    }
}

async fn start_rest_api(
    matchmaker: matchmaker::SharedMatchMaker,
    scheduler: job_scheduler::SharedJobScheduler
) -> Result<(), Box<dyn std::error::Error>> {
    use warp::{Filter, http::Response};
    use serde_json::json;
    
    // Route for submitting a new job
    let submit_job = warp::path!(\"jobs\")
        .and(warp::post())
        .and(warp::body::json())
        .map(move |payload: serde_json::Value| {
            let mut scheduler = scheduler.lock().unwrap();
            
            // Extract payload fields
            let command = payload.get(\"command\")
                .and_then(|v| v.as_str())
                .unwrap_or(\"default_command\")
                .to_string();
            
            let args = payload.get(\"args\")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_else(Vec::new);
            
            let input_data = payload.get(\"input_data\")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let env_vars = payload.get(\"env_vars\")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| {
                            v.as_str().map(|s| (k.clone(), s.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_else(std::collections::HashMap::new);
            
            let priority = payload.get(\"priority\")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u8;
            
            let user_id = payload.get(\"user_id\")
                .and_then(|v| v.as_str())
                .unwrap_or(\"anonymous\")
                .to_string();
            
            // Create job payload
            let job_payload = distributed_engine::JobPayload {
                command,
                args,
                input_data,
                env_vars,
            };
            
            // Submit to scheduler
            let job_id = scheduler.submit_job(job_payload, user_id, priority);
            
            // Return job ID
            Response::builder()
                .status(200)
                .body(json!({
                    \"job_id\": job_id,
                    \"status\": \"scheduled\"
                }).to_string())
                .unwrap()
        });
    
    // Route for getting job status
    let get_job_status = warp::path!("jobs" / u64)
        .and(warp::get())
        .map(move |job_id: u64| {
            let scheduler = scheduler.lock().unwrap();
            
            match scheduler.get_job_status(job_id) {
                Some(job) => {
                    Response::builder()
                        .status(200)
                        .body(serde_json::to_string(job).unwrap())
                        .unwrap()
                }
                None => {
                    Response::builder()
                        .status(404)
                        .body(json!({
                            \"error\": \"Job not found\"
                        }).to_string())
                        .unwrap()
                }
            }
        });
    
    // Route for listing all jobs
    let list_jobs = warp::path!("jobs")
        .and(warp::get())
        .map(move || {
            let scheduler = scheduler.lock().unwrap();
            let jobs = scheduler.get_all_jobs();
            
            Response::builder()
                .status(200)
                .body(serde_json::to_string(&jobs).unwrap())
                .unwrap()
        });
    
    // Route for cancelling a job
    let cancel_job = warp::path!("jobs" / u64 / "cancel")
        .and(warp::post())
        .map(move |job_id: u64| {
            let mut scheduler = scheduler.lock().unwrap();
            let success = scheduler.cancel_job(job_id);
            
            if success {
                Response::builder()
                    .status(200)
                    .body(json!({
                        \"job_id\": job_id,
                        \"status\": \"cancelled\"
                    }).to_string())
                    .unwrap()
            } else {
                Response::builder()
                    .status(404)
                    .body(json!({
                        \"error\": \"Job not found\"
                    }).to_string())
                    .unwrap()
            }
        });
    
    // Serve healthcheck endpoint
    let healthcheck = warp::path!("health")
        .map(|| {
            Response::builder()
                .status(200)
                .body(json!({
                    \"status\": \"healthy\",
                    \"version\": \"1.0.0\"
                }).to_string())
                .unwrap()
        });
    
    // Combined routes
    let routes = submit_job
        .or(get_job_status)
        .or(list_jobs)
        .or(cancel_job)
        .or(healthcheck);
    
    // Start the server
    warp::serve(routes)
        .run(([127, 0, 0, 1], 9002))
        .await;
    
    Ok(())
}
