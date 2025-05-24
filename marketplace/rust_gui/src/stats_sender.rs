use std::thread;
use std::time::Duration;
use serde_json::json;
use sysinfo::{System, SystemExt};

pub struct StatsSender {
    ws_url: String,
    node_id: String,
    interval_ms: u64,
}

impl StatsSender {
    pub fn new(ws_url: &str, node_id: &str, interval_ms: u64) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            node_id: node_id.to_string(),
            interval_ms,
        }
    }
    
    pub fn start(&self) {
        let node_id = self.node_id.clone();
        let interval_ms = self.interval_ms;
        
        thread::spawn(move || {
            loop {
                // Create a new system info collector
                let mut sys = System::new_all();
                sys.refresh_all();
                
                // Get CPU and memory usage
                let cpu_usage = sys.global_cpu_info().cpu_usage();
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
                    "node_id": node_id,
                    "stats": {
                        "cpu_usage": cpu_usage,
                        "memory_usage": memory_usage,
                        "available": true,
                        "timestamp": chrono::Utc::now().timestamp()
                    }
                });
                
                // Try to send stats via HTTP as fallback
                let _ = ureq::post("http://127.0.0.1:8000/nodes/stats")
                    .content_type("application/json")
                    .send(json_payload.to_string());
                
                // Sleep for the specified interval
                thread::sleep(Duration::from_millis(interval_ms));
            }
        });
    }
}

