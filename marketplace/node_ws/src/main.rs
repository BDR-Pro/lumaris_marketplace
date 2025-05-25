mod api_client;
mod matchmaker;
mod ws_handler;
mod vm_manager;
mod buyer_stats;
mod seller_stats;

use log::{info, error};
use dotenv::dotenv;
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;
use colored::*;

use ws_handler::run_ws_server;
use matchmaker::create_matchmaker;
use vm_manager::VmManager;
use buyer_stats::BuyerStatsManager;
use seller_stats::SellerStatsManager;

#[tokio::main]
async fn main() {
    // Initialize environment variables
    dotenv().ok();
    
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    // Print banner with colored output
    println!("{}", "
    ██╗     ██╗   ██╗███╗   ███╗ █████╗ ██████╗ ██╗███████╗
    ██║     ██║   ██║████╗ ████║██╔══██╗██╔══██╗██║██╔════╝
    ██║     ██║   ██║██╔████╔██║███████║██████╔╝██║███████╗
    ██║     ██║   ██║██║╚██╔╝██║██╔══██║██╔══██╗██║╚════██║
    ███████╗╚██████╔╝██║ ╚═╝ ██║██║  ██║██║  ██║██║███████║
    ╚══════╝ ╚═════╝ ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚══════╝
    ".bright_blue());
    
    println!("{}", "    Node WebSocket Server".bright_green().bold());
    println!("{}", "    =====================".bright_green());
    
    // Get API URL from environment variables
    // #ignore-devskim: DS137138 - This is a fallback for local development only
    let api_url = env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    info!("{} {}", "API URL:".bright_yellow(), api_url.bright_white());
    
    // Get VM base path from environment variables
    let vm_base_path = env::var("VM_BASE_PATH").unwrap_or_else(|_| "/tmp/lumaris/vms".to_string());
    info!("{} {}", "VM Base Path:".bright_yellow(), vm_base_path.bright_white());
    
    // Create the matchmaker
    let matchmaker = create_matchmaker();
    info!("{}", "✓ Matchmaker initialized".green());
    
    // Create the VM manager
    let vm_manager = VmManager::new(&vm_base_path, &api_url);
    info!("{}", "✓ VM Manager initialized".green());
    
    // Create the buyer statistics manager
    let buyer_stats_manager = BuyerStatsManager::new();
    info!("{}", "✓ Buyer Statistics Manager initialized".green());
    
    // Create the seller statistics manager
    let seller_stats_manager = SellerStatsManager::new();
    info!("{}", "✓ Seller Statistics Manager initialized".green());
    
    // Wait for API to be ready
    info!("{}", "Waiting for API to be ready...".bright_cyan());
    let mut api_ready = false;
    for i in 0..30 {
        match reqwest::get(&format!("{}/health", api_url)).await {
            Ok(response) if response.status().is_success() => {
                api_ready = true;
                info!("{}", "✓ API is ready!".green().bold());
                break;
            },
            _ => {
                print!("{}", ".".bright_cyan());
                if i % 10 == 9 {
                    println!();
                }
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    if !api_ready {
        error!("{}", "⚠ API not ready after 30 seconds, continuing anyway...".red().bold());
    }
    
    // Start WebSocket server for node connections
    info!("{}", "🔄 Starting WebSocket Server on 0.0.0.0:3030 (WS)...".bright_green().bold());
    let connections: Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>> = Arc::new(Mutex::new(HashMap::new()));
    if let Err(e) = run_ws_server("0.0.0.0:3030", &api_url, matchmaker.clone(), &connections, buyer_stats_manager, seller_stats_manager).await {
        error!("{} {}", "WebSocket server error:".red().bold(), e);
    }
}
