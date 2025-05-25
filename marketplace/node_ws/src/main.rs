mod api_client;
mod matchmaker;
mod ws_handler;
mod vm_manager;

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
use vm_manager::VmManager;

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
    // #ignore-devskim: DS137138 - This is a fallback for local development only
    let api_url = env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    info!("API URL: {}", api_url);
    
    // Get VM base path from environment variables
    let vm_base_path = env::var("VM_BASE_PATH").unwrap_or_else(|_| "/tmp/lumaris/vms".to_string());
    info!("VM Base Path: {}", vm_base_path);
    
    // Create the matchmaker
    let matchmaker = create_matchmaker();
    
    // Create the VM manager
    let vm_manager = VmManager::new(&vm_base_path, &api_url);
    info!("VM Manager initialized");
    
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
    if let Err(e) = run_ws_server("0.0.0.0:3030", &api_url, matchmaker.clone(), &connections).await {
        error!("WebSocket server error: {}", e);
    }
}

