use std::thread;
use std::time::Duration;
use serde_json::json;
use sysinfo::System;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

pub struct StatsSender {
    ws_url: String,
    node_id: String,
    interval: Duration,
}

impl StatsSender {
    pub fn new(ws_url: &str, node_id: &str, interval_ms: u64) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            node_id: node_id.to_string(),
            interval: Duration::from_millis(interval_ms),
        }
    }
    
    pub fn start(&self) {
        let ws_url = self.ws_url.clone();
        let node_id = self.node_id.clone();
        let interval = self.interval;
        
        thread::spawn(move || {
            // Initialize system info collector
            let mut sys = System::new_all();
            
            loop {
                // Try to connect to WebSocket server
                if let Ok(url) = Url::parse(&ws_url) {
                    // Use tokio runtime for async WebSocket
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    
                    rt.block_on(async {
                        match connect_async(url).await {
                            Ok((mut ws_stream, _)) => {
                                // Refresh system information
                                sys.refresh_all();
                                
                                // Get CPU usage
                                let cpu_usage = sys.global_cpu_info().cpu_usage();
                                
                                // Get memory usage
                                let used_memory = sys.used_memory();
                                let total_memory = sys.total_memory();
                                
                                // Get hostname
                                let hostname = hostname::get()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                
                                // Create stats message
                                let json = json!({
                                    "type": "node_stats",
                                    "node_id": node_id,
                                    "hostname": hostname,
                                    "cpu_usage": cpu_usage,
                                    "memory_used": used_memory,
                                    "memory_total": total_memory,
                                    "timestamp": chrono::Utc::now().timestamp()
                                }).to_string();
                                
                                // Send stats via WebSocket
                                let _ = ws_stream.send(Message::Text(json.into())).await;
                                
                                // Also send via HTTP as fallback
                                let _ = ureq::post("http://127.0.0.1:8000/nodes/stats")
                                    .content_type("application/json")
                                    .send_string(&json);
                            },
                            Err(e) => {
                                eprintln!("Failed to connect to WebSocket server: {}", e);
                            }
                        }
                    });
                }
                
                // Wait for next update
                thread::sleep(interval);
            }
        });
    }
}
