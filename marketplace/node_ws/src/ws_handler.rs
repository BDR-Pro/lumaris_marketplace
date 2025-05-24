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
pub async fn run_ws_server(addr: &str, matchmaker: SharedMatchMaker) -> Result<()> {
    info!("🚀 WebSocket server listening on {}", addr);
    
    // Store active connections
    let connections: NodeConnections = Arc::new(Mutex::new(HashMap::new()));
    
    // Clone for event listener
    let connections_for_events = connections.clone();
    
    // Create a broadcast channel for matchmaker events
    let (tx, rx) = broadcast::channel::<String>(100);
    
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
    let listener = TcpListener::bind(addr).await?;
    
    // Accept connections
    while let Ok((stream, addr)) = listener.accept().await {
        info!("New connection from: {}", addr);
        let peer = format!("{}", addr);
        let matchmaker_clone = matchmaker.clone();
        let connections_clone = connections.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_websocket_connection(stream, peer, matchmaker_clone, connections_clone).await {
                error!("Error processing connection: {:?}", e);
            }
        });
    }
    
    Ok(())
}

// Handle a WebSocket connection
async fn handle_websocket(
    socket: warp::ws::WebSocket,
    matchmaker: SharedMatchMaker,
    mut rx: broadcast::Receiver<String>
) {
    info!("New WebSocket connection established");
    
    let (mut tx, mut rx_ws) = socket.split();
    
    // Handle incoming messages
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
                                    "node_status" => {
                                        // Update node status in the matchmaker
                                        if let (Some(node_id), Some(available)) = (
                                            json.get("node_id").and_then(|n| n.as_str()),
                                            json.get("available").and_then(|a| a.as_bool())
                                        ) {
                                            let mut mm = matchmaker.lock().unwrap();
                                            mm.update_node_availability(node_id.to_string(), available);
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

// Handle a single WebSocket connection
async fn handle_websocket_connection(
    stream: TcpStream,
    peer_id: String,
    matchmaker: SharedMatchMaker,
    connections: NodeConnections
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
                            process_message(&text, &peer_id, &matchmaker).await?;
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

// Process incoming WebSocket messages
async fn process_message(
    message: &str,
    peer_id: &str,
    matchmaker: &SharedMatchMaker
) -> Result<()> {
    // Parse the message
    let parsed: Value = serde_json::from_str(message)
        .map_err(|e| NodeError::SerializationError(e))?;
    
    // Extract message type
    let msg_type = parsed["type"].as_str().unwrap_or("unknown");
    
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
                let mut matchmaker = matchmaker.lock().unwrap();
                matchmaker.update_node(node.clone());
            }
            
            // Update node availability in the API
            let peer_id_clone = peer_id.to_string();
            tokio::spawn(async move {
                if let Err(e) = update_node_availability("http://localhost:8000", &peer_id_clone, true).await {
                    error!("Failed to update node availability: {:?}", e);
                }
            });
        }
        "node_status" => {
            // Update node status
            let cpu_usage = parsed["status"]["cpu_usage"].as_f64().unwrap_or(0.0) as f32;
            let memory_usage = parsed["status"]["memory_usage"].as_f64().unwrap_or(0.0) as f32;
            let available = parsed["status"]["available"].as_bool().unwrap_or(true);
            
            {
                let mut matchmaker = matchmaker.lock().unwrap();
                if let Some(node) = matchmaker.get_node_by_id_mut(peer_id) {
                    // Update node capabilities based on current usage
                    node.available = available;
                    node.last_updated = Utc::now().timestamp() as u64;
                    
                    debug!("Updated node {} status: CPU {}%, Memory {}%, Available: {}", 
                        peer_id, cpu_usage, memory_usage, available);
                }
            }
        }
        "job_status" => {
            // Update job status
            let job_id = parsed["job_id"].as_u64().unwrap_or(0);
            let status_str = parsed["status"].as_str().unwrap_or("unknown");
            
            let status = match status_str {
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
            
            {
                let mut matchmaker = matchmaker.lock().unwrap();
                matchmaker.update_job_status(job_id, status.clone());
            }
            
            // Update job status in the API
            let job_id_copy = job_id;
            let status_str_copy = status_str.to_string();
            tokio::spawn(async move {
                if let Err(e) = update_job_status("http://localhost:8000", job_id_copy, &status_str_copy, result_data).await {
                    error!("Failed to update job status: {:?}", e);
                }
            });
        }
        _ => {
            debug!("Unknown message type: {}", msg_type);
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
    let (tx, rx) = broadcast::channel::<String>(100);
    
    // Store the sender in the matchmaker
    {
        let mut mm = matchmaker.lock().unwrap();
        mm.set_tx(tx);
    }
    
    // Create the WebSocket handler
    warp::path("ws")
        .and(warp::ws())
        .and(warp::any().map(move || {
            let rx_clone = rx.resubscribe();
            (Arc::clone(&matchmaker), rx_clone)
        }))
        .map(|ws: warp::ws::Ws, (matchmaker, rx)| {
            ws.on_upgrade(move |socket| handle_websocket(socket, matchmaker, rx))
        })
}
