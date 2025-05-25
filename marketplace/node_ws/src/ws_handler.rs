// File: marketplace/node_ws/src/ws_handler.rs (updated)

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tokio::sync::broadcast;
use warp::Filter;
use log::{info, error, debug};
use serde_json::{json, Value};
use chrono::Utc;

use crate::matchmaker::{
    SharedMatchMaker, NodeCapabilities, 
    JobStatus, MatchmakerMessage
};
use crate::http_client::{update_node_availability, update_job_status};
use crate::error::{NodeError, Result};

// Type aliases for clarity
type NodeConnections = Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>;
type WsResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Main WebSocket server function
pub async fn run_ws_server(addr: &str, port: u16, matchmaker: SharedMatchMaker) -> Result<()> {
    let addr = format!("{}:{}", addr, port);
    info!("🚀 WebSocket server listening on {}", addr);
    
    // Store active connections
    let connections: NodeConnections = Arc::new(Mutex::new(HashMap::new()));
    
    // Clone for event listener
    let _connections_for_events = connections.clone();
    
    // Create a broadcast channel for matchmaker events
    let (tx, _rx) = broadcast::channel::<String>(100);
    
    // Listen for matchmaker events
    tokio::spawn(async move {
        // In a real implementation, you would listen for events from the matchmaker
        // and broadcast them to all connected clients
        loop {
            sleep(Duration::from_secs(10)).await;
            let _ = tx.send(json!({
                "type": "heartbeat",
                "timestamp": Utc::now().timestamp()
            }).to_string());
        }
    });
    
    // Create a TCP listener
    let listener = TcpListener::bind(&addr).await?;
    
    // Accept connections
    while let Ok((stream, addr)) = listener.accept().await {
        info!("New connection from: {}", addr);
        let peer = format!("{}", addr);
        
        // Handle the WebSocket connection
        tokio::spawn(handle_tcp_connection(stream, peer, connections.clone(), matchmaker.clone()));
    }
    
    Ok(())
}

// Handle a single WebSocket connection
async fn handle_tcp_connection(
    stream: TcpStream,
    peer_id: String,
    connections: NodeConnections,
    matchmaker: SharedMatchMaker
) -> WsResult<()> {
    // Upgrade the TCP stream to a WebSocket connection
    let ws_stream = accept_async(stream).await?;
    info!("WebSocket connection established with: {}", peer_id);
    
    // Split the WebSocket stream into sender and receiver
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Create a channel for sending messages to this client
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    
    // Store the sender in our connections map
    {
        let mut connections = connections.lock().await;
        connections.insert(peer_id.clone(), tx);
    }
    
    // Send a welcome message
    let welcome_msg = Message::Text(json!({
        "type": "welcome",
        "message": "Connected to Lumaris Marketplace Node WebSocket",
        "timestamp": Utc::now().timestamp()
    }).to_string().into());
    
    ws_sender.send(welcome_msg).await?;
    
    // Forward messages from the channel to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            ws_sender.send(msg).await?;
        }
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    
    // Process incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(msg) => {
                    match msg {
                        Message::Text(text) => {
                            process_message(&text, &peer_id, &matchmaker, &connections).await?;
                        }
                        Message::Close(_) => {
                            info!("Connection closed by client: {}", peer_id);
                            break;
                        }
                        _ => {} // Ignore other message types
                    }
                }
                Err(e) => {
                    error!("Error receiving message from {}: {}", peer_id, e);
                    break;
                }
            }
        }
        
        // Remove connection when client disconnects
        let mut connections = connections.lock().await;
        connections.remove(&peer_id);
        info!("Removed connection: {}", peer_id);
        
        // Mark node as unavailable
        {
            let mut matchmaker = matchmaker.lock().unwrap();
            if let Some(node) = matchmaker.get_node_by_id_mut(&peer_id) {
                node.available = false;
                info!("Marked node {} as unavailable", peer_id);
                
                // Update availability in the API
                let peer_id_clone = peer_id.to_string();
                tokio::spawn(async move {
                    if let Err(e) = update_node_availability("http://localhost:8000", &peer_id_clone, false).await {
                        error!("Failed to update node availability: {:?}", e);
                    }
                });
            }
        }
        
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    
    // Wait for either task to complete
    tokio::select! {
        res = &mut send_task => {
            if let Err(e) = res {
                error!("Send task error: {:?}", e);
            }
        }
        res = &mut recv_task => {
            if let Err(e) = res {
                error!("Receive task error: {:?}", e);
            }
        }
    }
    
    Ok(())
}

// Handle a WebSocket connection
async fn handle_websocket_connection(
    socket: warp::ws::WebSocket,
    matchmaker: SharedMatchMaker,
    mut rx: broadcast::Receiver<String>
) {
    info!("New WebSocket connection established");
    
    let (mut tx, mut rx_ws) = socket.split();
    
    // Handle incoming messages
    let matchmaker_clone = matchmaker.clone();
    tokio::task::spawn(async move {
        while let Some(result) = rx_ws.next().await {
            match result {
                Ok(msg) => {
                    if let Ok(text) = msg.to_str() {
                        info!("Received message: {}", text);
                        
                        // Parse the message as JSON
                        if let Ok(json) = serde_json::from_str::<Value>(text) {
                            // Process the message based on its type
                            if let Some(msg_type) = json.get("type").and_then(|t| t.as_str()) {
                                match msg_type {
                                    "node_registration" => {
                                        // Handle node registration
                                        if let Some(node_id) = json.get("node_id").and_then(|n| n.as_str()) {
                                            let cpu_cores = json.get("cpu_cores").and_then(|c| c.as_f64()).unwrap_or(1.0) as f32;
                                            let memory_mb = json.get("memory_mb").and_then(|m| m.as_u64()).unwrap_or(1024);
                                            
                                            let node = NodeCapabilities {
                                                node_id: node_id.to_string(),
                                                cpu_cores,
                                                memory_mb,
                                                available: true,
                                                reliability_score: 1.0,
                                                last_updated: Utc::now().timestamp() as u64,
                                            };
                                            
                                            let mut mm = matchmaker_clone.lock().unwrap();
                                            mm.register_node(node);
                                        }
                                    },
                                    "node_status" => {
                                        // Update node status
                                        let cpu_usage = json.get("status").and_then(|s| s.get("cpu_usage").and_then(|c| c.as_f64())).unwrap_or(0.0) as f32;
                                        let memory_usage = json.get("status").and_then(|s| s.get("memory_usage").and_then(|m| m.as_f64())).unwrap_or(0.0) as f32;
                                        let available = json.get("status").and_then(|s| s.get("available").and_then(|a| a.as_bool())).unwrap_or(true);
                                        
                                        {
                                            let mut matchmaker = matchmaker_clone.lock().unwrap();
                                            if let Some(node) = matchmaker.get_node_by_id_mut(peer_id) {
                                                // Update node capabilities based on current usage
                                                node.available = available;
                                                node.last_updated = Utc::now().timestamp() as u64;
                                                
                                                debug!("Updated node {} status: CPU {}%, Memory {}%, Available: {}", 
                                                    peer_id, cpu_usage, memory_usage, available);
                                            }
                                        }
                                    },
                                    "job_status_update" => {
                                        // Extract job ID and status
                                        if let (Some(job_id), Some(status)) = (
                                            json.get("job_id").and_then(|j| j.as_u64()),
                                            json.get("status").and_then(|s| s.as_str())
                                        ) {
                                            // Update job status
                                            let mut mm = matchmaker_clone.lock().unwrap();
                                            let _ = mm.update_job_status(job_id, status.to_string());
                                        }
                                    },
                                    _ => {
                                        info!("Unknown message type: {}", msg_type);
                                    }
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("Error receiving message: {}", e);
                    break;
                }
            }
        }
    });
    
    // Forward broadcast messages to this WebSocket
    tokio::task::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Err(e) = tx.send(warp::ws::Message::text(msg)).await {
                error!("Error sending message: {}", e);
                break;
            }
        }
    });
}

// Process incoming WebSocket messages
async fn process_message(
    message: &str,
    peer_id: &str,
    matchmaker: &SharedMatchMaker,
    connections: &NodeConnections
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                    mm.register_node(node.clone());
                }
            },
            "job_status_update" => {
                // Extract job ID and status
                if let (Some(job_id), Some(status)) = (
                    parsed.get("job_id").and_then(|j| j.as_u64()),
                    parsed.get("status").and_then(|s| s.as_str())
                ) {
                    // Update job status
                    let mut mm = matchmaker.lock().unwrap();
                    let _ = mm.update_job_status(job_id, status.to_string());
                }
            },
            _ => {
                debug!("Unknown message type: {}", msg_type);
            }
        }
    }
    
    Ok(())
}

// Handle job status updates and broadcast to connected clients
async fn handle_job_status_update(
    job_id: u64,
    status: JobStatus,
    connections: &NodeConnections
) -> WsResult<()> {
    // Get status as string
    let status_str = match status {
        JobStatus::Queued => "queued",
        JobStatus::Matching => "matching",
        JobStatus::Assigned => "assigned",
        JobStatus::Running => "running",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
    };
    
    // Create the update message
    let update_msg = serde_json::json!({
        "type": "job_status_update",
        "job_id": job_id,
        "status": status_str
    }).to_string();
    
    let ws_msg = Message::Text(update_msg.into());
    
    // Broadcast to all connected clients
    let connections = connections.lock().await;
    for (_, tx) in connections.iter() {
        let _ = tx.send(ws_msg.clone());
    }
    
    Ok(())
}

// Create a WebSocket handler for the matchmaker
pub fn create_ws_handler(matchmaker: SharedMatchMaker) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Create a channel for broadcasting messages
    let (tx, _rx) = broadcast::channel::<String>(100);
    
    // Store the sender in the matchmaker
    {
        let mut mm = matchmaker.lock().unwrap();
        mm.set_tx(tx.clone());
    }
    
    // Create the WebSocket handler
    warp::path("ws")
        .and(warp::ws())
        .and(with_matchmaker(matchmaker))
        .and(with_broadcaster(tx))
        .map(|ws: warp::ws::Ws, matchmaker: SharedMatchMaker, tx: broadcast::Sender<String>| {
            ws.on_upgrade(move |socket| {
                let rx = tx.subscribe();
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
