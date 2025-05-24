// File: marketplace/node_ws/src/ws_handler.rs (updated)

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use serde_json::{Value, json};
use chrono::Utc;
use crate::http_client::{update_node_availability, update_job_status};
use crate::error::{Result, NodeError};
use log::{info, error, debug, warn};

// Import our new matchmaker
use crate::matchmaker::{
    SharedMatchMaker, create_matchmaker, NodeCapabilities, 
    Job, JobRequirements, JobStatus, MatchmakerMessage
};

// Type aliases for clarity
type NodeConnections = Arc<TokioMutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>;
type WsResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Main WebSocket server function
pub async fn start_ws_server(host: &str, port: u16, matchmaker: SharedMatchMaker) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("🚀 WebSocket server listening on {}", addr);
    
    // Store active connections
    let connections: NodeConnections = Arc::new(TokioMutex::new(HashMap::new()));
    
    // Clone for event listener
    let connections_for_events = connections.clone();
    
    // Listen for matchmaker events
    let (_, mut rx) = {
        let matchmaker_guard = matchmaker.lock().unwrap();
        (Arc::clone(&matchmaker), rx)
    };
    
    // Spawn a task to handle matchmaker events
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            match msg {
                MatchmakerMessage::JobStatusUpdate(job_id, status) => {
                    if let Err(e) = handle_job_status_update(job_id, status, &connections_for_events).await {
                        error!("Failed to handle job status update: {:?}", e);
                    }
                }
                _ => {} // Handle other message types as needed
            }
        }
    });

    // Accept incoming WebSocket connections
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
    }).to_string());
    
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
                tokio::spawn(async move {
                    if let Err(e) = update_node_availability("http://localhost:8000", &peer_id, false).await {
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
            tokio::spawn(async move {
                if let Err(e) = update_node_availability("http://localhost:8000", peer_id, true).await {
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
    
    let ws_msg = Message::Text(update_msg);
    
    // Broadcast to all connected clients
    let connections = connections.lock().await;
    for (_, tx) in connections.iter() {
        let _ = tx.send(ws_msg.clone());
    }
    
    Ok(())
}
