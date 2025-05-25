// node_ws/src/ws_handler.rs
// WebSocket handler for node communication

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use warp::Filter;
use log::{info, error, debug};
use serde_json::{json, Value};
use chrono::Utc;
use warp::ws::{Message, WebSocket};
use uuid::Uuid;

use crate::error::Result;
use crate::http_client::{update_node_availability, update_job_status};
use crate::matchmaker::{
    SharedMatchMaker, NodeCapabilities,
    JobStatus
};

// Type alias for WebSocket result
type WsResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Type alias for node connections
type NodeConnections = Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>>;

// Start the WebSocket server
pub async fn run_ws_server(
    addr: &str,
    matchmaker: SharedMatchMaker,
    _connections: &NodeConnections
) -> Result<()> {
    // Create the WebSocket handler
    let ws_route = create_ws_handler(matchmaker.clone());
    
    // Create a health check route
    let health_route = warp::path("health")
        .map(|| "OK");
    
    // Combine the routes
    let routes = ws_route.or(health_route);
    
    // Start the server
    info!("Starting WebSocket server on {}", addr);
    warp::serve(routes)
        .run(([0, 0, 0, 0], 3030))
        .await;
    
    Ok(())
}

// Handle a WebSocket connection
async fn handle_websocket_connection(
    ws: WebSocket,
    matchmaker: SharedMatchMaker,
    _rx: broadcast::Receiver<String>
) -> WsResult<()> {
    // Split the WebSocket into a sender and receiver
    let (ws_tx, mut ws_rx) = ws.split();
    
    // Generate a unique ID for this connection
    let peer_id = Uuid::new_v4().to_string();
    info!("New WebSocket connection: {}", peer_id);
    
    // Create a channel for sending messages to this WebSocket
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    
    // Store the sender in the connections map
    let connections = Arc::new(Mutex::new(HashMap::new()));
    {
        let mut conns = connections.lock().unwrap();
        conns.insert(peer_id.clone(), tx.clone());
    }
    
    // Clone for use in the task
    let matchmaker_clone = matchmaker.clone();
    let peer_id_clone = peer_id.clone();
    let ws_tx_clone = ws_tx.clone();
    
    // Handle incoming WebSocket messages
    tokio::task::spawn(async move {
        while let Some(result) = ws_rx.next().await {
            match result {
                Ok(msg) => {
                    // Skip if not a text message
                    if !msg.is_text() {
                        continue;
                    }
                    
                    // Get the message text
                    let msg_text = msg.to_str().unwrap_or_default();
                    
                    // Process the message
                    if let Err(e) = process_message(msg_text, &peer_id_clone, &matchmaker_clone, &connections).await {
                        error!("Error processing message: {}", e);
                        
                        // Send error response
                        let error_response = json!({
                            "type": "error",
                            "message": format!("Error processing message: {}", e)
                        }).to_string();
                        
                        if let Err(e) = ws_tx_clone.send(Message::text(error_response)).await {
                            error!("Error sending error response: {}", e);
                            break;
                        }
                    }
                },
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }
        
        // WebSocket closed, remove from connections
        {
            let mut conns = connections.lock().unwrap();
            conns.remove(&peer_id_clone);
        }
        
        info!("WebSocket connection closed: {}", peer_id_clone);
    });
    
    // Forward broadcast messages to this WebSocket
    let ws_tx_forward = ws_tx.clone();
    tokio::task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_tx_forward.send(Message::text(msg.clone())).await {

                error!("Error sending message: {}", e);
                break;
            }
        }
    });
    
    Ok(())
}

// Process incoming WebSocket messages
async fn process_message(
    message: &str,
    peer_id: &str,
    matchmaker: &SharedMatchMaker,
    _connections: &Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>>
) -> WsResult<()> {
    // Parse the message as JSON
    let parsed: Value = serde_json::from_str(message)?;
    
    // Process the message based on its type
    if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
        match msg_type {
            "node_registration" => {
                info!("Node registration from: {}", peer_id);
                
                // Extract node capabilities
                let cpu_cores = parsed["capabilities"]["cpu_cores"].as_f64().unwrap_or(1.0) as f32;
                let memory_mb = parsed["capabilities"]["memory_mb"].as_u64().unwrap_or(1024);
                
                let node = NodeCapabilities {
                    node_id: peer_id.to_string(),
                    cpu_cores,
                    memory_mb,
                    available: true,
                    reliability_score: 1.0, // Default for new nodes
                    last_updated: Utc::now().timestamp() as u64,
                };
                
                // Register the node with the matchmaker
                {
                    let mut mm = matchmaker.lock().unwrap();
                    mm.update_node(node.clone());
                }
            },
            "node_status" => {
                // Update node status
                let available = parsed["available"].as_bool().unwrap_or(true);
                
                {
                    let mut mm = matchmaker.lock().unwrap();
                    mm.update_node_availability(peer_id.to_string(), available);
                }
            },
            "job_status_update" => {
                // Extract job ID and status
                if let (Some(job_id), Some(status_str)) = (
                    parsed.get("job_id").and_then(|j| j.as_u64()),
                    parsed.get("status").and_then(|s| s.as_str())
                ) {
                    // Convert status string to enum
                    let _status = match status_str {
                        "queued" => JobStatus::Queued,
                        "matching" => JobStatus::Matching,
                        "assigned" => JobStatus::Assigned,
                        "running" => JobStatus::Running,
                        "completed" => JobStatus::Completed,
                        "failed" => JobStatus::Failed,
                        _ => JobStatus::Failed,
                    };
                    
                    // Extract result data if available
                    let result_data = parsed.get("result").cloned();
                    
                    // Update job status in matchmaker
                    {
                        let mut mm = matchmaker.lock().unwrap();
                        let _ = mm.update_job_status(job_id, status_str.to_string());
                    }
                    
                    // Update job status in the API
                    let job_id_copy = job_id;
                    let status_str_copy = status_str.to_string();
                    tokio::spawn(async move {
                        if let Err(e) = update_job_status("http://localhost:8000", job_id_copy, &status_str_copy, result_data).await {
                            error!("Failed to update job status: {}", e);
                        }
                    });
                }
            },
            _ => {
                info!("Unknown message type: {}", msg_type);
            }
        }
    }
    
    Ok(())
}

// Handle job status updates
async fn handle_job_status_update(
    job_id: u64,
    status_str: &str,
    connections: &Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>>
) -> WsResult<()> {
    // Create a status update message
    let update_msg = json!({
        "type": "job_status_update",
        "job_id": job_id,
        "status": status_str
    }).to_string();
    
    // Broadcast to all connected clients
    let conns = connections.lock().unwrap();
    for (_, tx) in conns.iter() {
        if let Err(e) = tx.send(update_msg.clone()) {
            error!("Error broadcasting job status update: {}", e);
        }
    }
    
    Ok(())
}

// Create a WebSocket handler for the matchmaker
pub fn create_ws_handler(
    matchmaker: SharedMatchMaker
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Create a broadcast channel for sending messages to all connected clients
    let (tx, _rx) = broadcast::channel(100);
    
    // Create the WebSocket route
    warp::path("ws")
        .and(warp::ws())
        .and(with_matchmaker(matchmaker))
        .and(with_broadcaster(tx))
        .map(|ws: warp::ws::Ws, matchmaker, tx| {
            ws.on_upgrade(move |socket| {
                // Create a new receiver for this connection
                let rx = tx.subscribe();
                
                // Handle the WebSocket connection
                handle_websocket_connection(socket, matchmaker, rx)
            })
        })
}

// Helper function to pass matchmaker to filter
fn with_matchmaker(matchmaker: SharedMatchMaker) -> impl Filter<Extract = (SharedMatchMaker,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || matchmaker.clone())
}

// Helper function to pass broadcaster to filter
fn with_broadcaster(tx: broadcast::Sender<String>) -> impl Filter<Extract = (broadcast::Sender<String>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || tx.clone())
}
