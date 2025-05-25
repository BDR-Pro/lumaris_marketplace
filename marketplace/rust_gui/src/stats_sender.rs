use std::thread;
use std::time::Duration;
use serde_json::json;
use sysinfo::System;
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use url::Url;
use std::sync::{Arc, Mutex};
use log::{info, error};

pub struct StatsSender {
    ws_url: String,
    user_id: String,
    interval_ms: u64,
    role: String,
    ws_sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>>,
}

impl StatsSender {
    pub fn new(ws_url: &str, user_id: &str, interval_ms: u64, role: String) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            user_id: user_id.to_string(),
            interval_ms,
            role,
            ws_sender: Arc::new(Mutex::new(None)),
        }
    }
    
    pub fn start(&self) {
        let ws_url = self.ws_url.clone();
        let user_id = self.user_id.clone();
        let interval_ms = self.interval_ms;
        let role = self.role.clone();
        let ws_sender = self.ws_sender.clone();
        
        thread::spawn(move || {
            // Create a new tokio runtime
            let rt = Runtime::new().unwrap();
            
            // Run the WebSocket connection in the runtime
            rt.block_on(async {
                // Create a channel for sending messages to the WebSocket
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
                
                // Store the sender in the shared state
                {
                    let mut sender = ws_sender.lock().unwrap();
                    *sender = Some(tx);
                }
                
                // Parse the WebSocket URL
                let url = match Url::parse(&ws_url) {
                    Ok(url) => url,
                    Err(e) => {
                        error!("Failed to parse WebSocket URL: {}", e);
                        return;
                    }
                };
                
                // Connect to the WebSocket server
                let (ws_stream, _) = match connect_async(url).await {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("Failed to connect to WebSocket server: {}", e);
                        return;
                    }
                };
                
                info!("Connected to WebSocket server");
                
                // Split the WebSocket stream
                let (mut write, mut read) = ws_stream.split();
                
                // Spawn a task to forward messages from the channel to the WebSocket
                let forward_task = tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        let json_str = serde_json::to_string(&msg).unwrap();
                        if let Err(e) = write.send(Message::Text(json_str)).await {
                            error!("Failed to send message to WebSocket: {}", e);
                            break;
                        }
                    }
                });
                
                // Spawn a task to read messages from the WebSocket
                let read_task = tokio::spawn(async move {
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                info!("Received message: {}", text);
                                // Process the message here
                            }
                            Ok(Message::Close(_)) => {
                                info!("WebSocket connection closed");
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                });
                
                // Wait for both tasks to complete
                let _ = tokio::join!(forward_task, read_task);
            });
        });
        
        // Start a thread to periodically send system stats
        if self.role == "seller" {
            let user_id = self.user_id.clone();
            let interval_ms = self.interval_ms;
            let ws_sender = self.ws_sender.clone();
            
            thread::spawn(move || {
                loop {
                    // Create a new system info collector
                    let mut sys = System::new_all();
                    sys.refresh_all();
                    
                    // Get CPU and memory usage
                    let cpu_usage = sys.global_cpu_usage();
                    let total_memory = sys.total_memory();
                    let used_memory = sys.used_memory();
                    let memory_usage = if total_memory > 0 {
                        (used_memory as f32 / total_memory as f32) * 100.0
                    } else {
                        0.0
                    };
                    
                    // Create JSON payload
                    let json_payload = json!({
                        "type": "node_stats",
                        "node_id": user_id,
                        "seller_id": user_id,
                        "stats": {
                            "cpu_usage": cpu_usage,
                            "memory_usage": memory_usage,
                            "available": true,
                            "timestamp": chrono::Utc::now().timestamp()
                        }
                    });
                    
                    // Send the stats via WebSocket
                    if let Some(sender) = ws_sender.lock().unwrap().as_ref() {
                        let _ = sender.send(json_payload);
                    }
                    
                    // Sleep for the specified interval
                    thread::sleep(Duration::from_millis(interval_ms));
                }
            });
        }
    }
    
    // Send a VM creation request
    pub fn send_create_vm(&self, job_id: u64, vcpu_count: u32, mem_size_mib: u32) {
        let json_payload = json!({
            "type": "create_vm",
            "job_id": job_id,
            "buyer_id": self.user_id,
            "vcpu_count": vcpu_count,
            "mem_size_mib": mem_size_mib
        });
        
        self.send_message(json_payload);
    }
    
    // Send a node registration request
    pub fn send_node_registration(&self, cpu_cores: f32, memory_mb: u64) {
        let json_payload = json!({
            "type": "node_registration",
            "seller_id": self.user_id,
            "capabilities": {
                "cpu_cores": cpu_cores,
                "memory_mb": memory_mb
            }
        });
        
        self.send_message(json_payload);
    }
    
    // Send a request for buyer statistics
    pub fn send_get_buyer_stats(&self) {
        let json_payload = json!({
            "type": "get_buyer_stats",
            "buyer_id": self.user_id
        });
        
        self.send_message(json_payload);
    }
    
    // Send a request for seller statistics
    pub fn send_get_seller_stats(&self) {
        let json_payload = json!({
            "type": "get_seller_stats",
            "seller_id": self.user_id
        });
        
        self.send_message(json_payload);
    }
    
    // Helper method to send a message via WebSocket
    fn send_message(&self, payload: serde_json::Value) {
        if let Some(sender) = self.ws_sender.lock().unwrap().as_ref() {
            let _ = sender.send(payload);
        }
    }
}

