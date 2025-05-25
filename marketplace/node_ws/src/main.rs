// File: marketplace/node_ws/src/main.rs (updated)

mod error;
mod config;
mod http_client;
mod matchmaker;
mod ws_handler;
mod job_scheduler;
mod vm_manager;

use std::env;
use config::Config;
use log::{info, error};
use ws_handler::run_ws_server;
use matchmaker::create_matchmaker;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

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
    let (matchmaker, _) = create_matchmaker();
    info!("✅ Matchmaker initialized");
    
    // Start WebSocket server for node connections
    info!("🔄 Starting WebSocket Server on 0.0.0.0:3030 (WS)...");
    let connections: Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>> = Arc::new(Mutex::new(HashMap::new()));
    if let Err(e) = run_ws_server("0.0.0.0:3030", matchmaker.clone(), &connections).await {
        error!("WebSocket server error: {}", e);
    }
}
