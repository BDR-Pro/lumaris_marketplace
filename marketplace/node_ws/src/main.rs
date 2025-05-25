mod api_client;
mod matchmaker;
mod ws_handler;

use log::{info, error};
use dotenv::dotenv;
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;

use ws_handler::run_ws_server;
use matchmaker::create_matchmaker;

#[tokio::main]
async fn main() {
    // Initialize environment variables
    dotenv().ok();
    
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    // Print banner
    info!("
    ██╗     ██╗   ██╗███╗   ███╗ █████╗ ██████╗ ██╗███████╗
    ██║     ██║   ██║████╗ ████║██╔══██╗██╔══██╗██║██╔════╝
    ██║     ██║   ██║██╔████╔██║███████║██████╔╝██║███████╗
    ██║     ██║   ██║██║╚██╔╝██║██╔══██║██╔══██╗██║╚════██║
    ███████╗╚██████╔╝██║ ╚═╝ ██║██║  ██║██║  ██║██║███████║
    ╚══════╝ ╚═════╝ ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚══════╝
    
    Node WebSocket Server
    ");
    
    // Get API URL from environment variables
    let api_url = env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    info!("API URL: {}", api_url);
    
    // Create the matchmaker
    let matchmaker = create_matchmaker();
    
    // Wait for API to be ready
    info!("Waiting for API to be ready...");
    let mut api_ready = false;
    for _ in 0..30 {
        match reqwest::get(&format!("{}/health", api_url)).await {
            Ok(response) if response.status().is_success() => {
                api_ready = true;
                break;
            },
            _ => {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    if !api_ready {
        error!("API not ready after 30 seconds, continuing anyway...");
    }
    
    // Start WebSocket server for node connections
    info!("🔄 Starting WebSocket Server on 0.0.0.0:3030 (WS)...");
    let connections: Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>> = Arc::new(Mutex::new(HashMap::new()));
    if let Err(e) = run_ws_server("0.0.0.0:3030", matchmaker.clone(), &connections).await {
        error!("WebSocket server error: {}", e);
    }
}

