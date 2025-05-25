use std::thread;
use std::time::Duration;
use serde_json::json;
use sysinfo::System;
use tokio::runtime::Runtime;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use url::Url;
use std::sync::{Arc, Mutex};
use log::{info, error, warn};
use uuid::Uuid;

// User role enum
#[derive(Debug, Clone, PartialEq)]
pub enum UserRole {
    Buyer,
    Seller,
}

// Stats sender for the GUI
pub struct StatsSender {
    ws_url: String,
    user_id: String,
    interval_ms: u64,
    role: UserRole,
    connected: Arc<Mutex<bool>>,
    ws_sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>>,
}

impl StatsSender {
    // Create a new stats sender
    pub fn new(ws_url: &str, user_id: &str, interval_ms: u64, role: String) -> Self {
        let role = match role.as_str() {
            "buyer" => UserRole::Buyer,
            "seller" => UserRole::Seller,
            _ => UserRole::Buyer, // Default to buyer
        };
        
        Self {
            ws_url: ws_url.to_string(),
            user_id: user_id.to_string(),
            interval_ms,
            role,
            connected: Arc::new(Mutex::new(false)),
            ws_sender: Arc::new(Mutex::new(None)),
        }
    }
    
    // Start the stats sender
    pub fn start(&self) {
        let ws_url = self.ws_url.clone();
        let user_id = self.user_id.clone();
        let interval_ms = self.interval_ms;
        let role = self.role.clone();
        let connected = self.connected.clone();
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
                
                // Connect to the WebSocket server with retry
                let mut retry_count = 0;
                let max_retries = 5;
                let mut ws_stream = None;
                
                while retry_count < max_retries {
                    // Parse the WebSocket URL
                    let url = match Url::parse(&ws_url) {
                        Ok(url) => url,
                        Err(e) => {
                            error!("Failed to parse WebSocket URL: {}", e);
                            return;
                        }
                    };
                    
                    // Connect to the WebSocket server
                    match connect_async(url).await {
                        Ok((stream, _)) => {
                            info!("Connected to WebSocket server at {}", ws_url);
                            ws_stream = Some(stream);
                            
                            // Update connected status
                            {
                                let mut is_connected = connected.lock().unwrap();
                                *is_connected = true;
                            }
                            
                            break;
                        },
                        Err(e) => {
                            retry_count += 1;
                            error!("Failed to connect to WebSocket server (attempt {}/{}): {}", 
                                retry_count, max_retries, e);
                            
                            if retry_count < max_retries {
                                let delay = 1000 * 2u64.pow(retry_count - 1);
                                warn!("Retrying in {} ms...", delay);
                                tokio::time::sleep(Duration::from_millis(delay)).await;
                            } else {
                                error!("Maximum retry attempts reached, giving up");
                                return;
                            }
                        }
                    }
                }
                
                // If connection failed after all retries, return
                let ws_stream = match ws_stream {
                    Some(stream) => stream,
                    None => return,
                };
                
                // Split the WebSocket stream
                let (mut write, mut read) = ws_stream.split();
                
                // Send initial identification message
                let init_message = json!({
                    "type": "identify",
                    "request_id": Uuid::new_v4().to_string(),
                    "role": match role {
                        UserRole::Buyer => "buyer",
                        UserRole::Seller => "seller",
                    },
                    "data": {
                        "user_id": user_id,
                        "client_version": env!("CARGO_PKG_VERSION"),
                    },
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                });
                
                if let Err(e) = write.send(Message::Text(serde_json::to_string(&init_message).unwrap())).await {
                    error!("Failed to send identification message: {}", e);
                    return;
                }
                
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
                                
                                // Update connected status
                                {
                                    let mut is_connected = connected.lock().unwrap();
                                    *is_connected = false;
                                }
                                
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket error: {}", e);
                                
                                // Update connected status
                                {
                                    let mut is_connected = connected.lock().unwrap();
                                    *is_connected = false;
                                }
                                
                                break;
                            }
                            _ => {}
                        }
                    }
                });
                
                // Start heartbeat task
                let heartbeat_task = {
                    let connected = connected.clone();
                    let mut write = write.clone();
                    
                    tokio::spawn(async move {
                        loop {
                            tokio::time::sleep(Duration::from_secs(30)).await;
                            
                            // Check if still connected
                            let is_connected = {
                                let connected = connected.lock().unwrap();
                                *connected
                            };
                            
                            if !is_connected {
                                break;
                            }
                            
                            // Send heartbeat
                            let heartbeat = json!({
                                "type": "heartbeat",
                                "request_id": Uuid::new_v4().to_string(),
                                "role": match role {
                                    UserRole::Buyer => "buyer",
                                    UserRole::Seller => "seller",
                                },
                                "data": {
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                },
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });
                            
                            if let Err(e) = write.send(Message::Text(serde_json::to_string(&heartbeat).unwrap())).await {
                                error!("Failed to send heartbeat: {}", e);
                                break;
                            }
                        }
                    })
                };
                
                // Start system stats task for sellers
                if role == UserRole::Seller {
                    let user_id = user_id.clone();
                    let interval_ms = interval_ms;
                    let ws_sender = ws_sender.clone();
                    
                    tokio::spawn(async move {
                        loop {
                            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
                            
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
                                "request_id": Uuid::new_v4().to_string(),
                                "role": "seller",
                                "data": {
                                    "node_id": user_id,
                                    "seller_id": user_id,
                                    "stats": {
                                        "cpu_usage": cpu_usage,
                                        "memory_usage": memory_usage,
                                        "available": true,
                                        "timestamp": chrono::Utc::now().timestamp()
                                    }
                                },
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                            });
                            
                            // Send the stats via WebSocket
                            if let Some(sender) = ws_sender.lock().unwrap().as_ref() {
                                let _ = sender.send(json_payload);
                            }
                        }
                    });
                }
                
                // Wait for both tasks to complete
                let _ = tokio::join!(forward_task, read_task, heartbeat_task);
            });
        });
    }
    
    // Send a VM creation request
    pub fn send_create_vm(&self, job_id: u64, vcpu_count: u32, mem_size_mib: u32) {
        if self.role != UserRole::Buyer {
            error!("Only buyers can create VMs");
            return;
        }
        
        let json_payload = json!({
            "type": "create_vm",
            "request_id": Uuid::new_v4().to_string(),
            "role": "buyer",
            "data": {
                "job_id": job_id,
                "buyer_id": self.user_id,
                "vcpu_count": vcpu_count,
                "mem_size_mib": mem_size_mib
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        self.send_message(json_payload);
    }
    
    // Send a node registration request
    pub fn send_node_registration(&self, cpu_cores: f32, memory_mb: u64) {
        if self.role != UserRole::Seller {
            error!("Only sellers can register nodes");
            return;
        }
        
        let json_payload = json!({
            "type": "node_registration",
            "request_id": Uuid::new_v4().to_string(),
            "role": "seller",
            "data": {
                "seller_id": self.user_id,
                "capabilities": {
                    "cpu_cores": cpu_cores,
                    "memory_mb": memory_mb
                }
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        self.send_message(json_payload);
    }
    
    // Send a request for buyer statistics
    pub fn send_get_buyer_stats(&self) {
        if self.role != UserRole::Buyer {
            error!("Only buyers can get buyer statistics");
            return;
        }
        
        let json_payload = json!({
            "type": "get_buyer_stats",
            "request_id": Uuid::new_v4().to_string(),
            "role": "buyer",
            "data": {
                "buyer_id": self.user_id
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        self.send_message(json_payload);
    }
    
    // Send a request for seller statistics
    pub fn send_get_seller_stats(&self) {
        if self.role != UserRole::Seller {
            error!("Only sellers can get seller statistics");
            return;
        }
        
        let json_payload = json!({
            "type": "get_seller_stats",
            "request_id": Uuid::new_v4().to_string(),
            "role": "seller",
            "data": {
                "seller_id": self.user_id
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        self.send_message(json_payload);
    }
    
    // Helper method to send a message via WebSocket
    fn send_message(&self, payload: serde_json::Value) {
        if let Some(sender) = self.ws_sender.lock().unwrap().as_ref() {
            if let Err(e) = sender.send(payload) {
                error!("Failed to send message: {}", e);
            }
        } else {
            error!("WebSocket sender not initialized");
        }
    }
    
    // Check if connected to the WebSocket server
    pub fn is_connected(&self) -> bool {
        let connected = self.connected.lock().unwrap();
        *connected
    }
}
