// File: marketplace/node_ws/src/ws_handler.rs (updated)

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use serde_json::{Value, json};
use chrono::Utc;
use crate::http_client::update_node_availability;
use crate::error::{Result, NodeError};
use log::{info, error, debug, warn};

// Import our new matchmaker
use crate::matchmaker::{
    SharedMatchMaker, create_matchmaker, NodeCapabilities, 
    Job, JobRequirements, JobStatus, MatchmakerMessage
};

// Store active connections
type ConnectionId = String;
type NodeConnections = Arc<TokioMutex<HashMap<ConnectionId, tokio::sync::mpsc::Sender<Message>>>>;

pub async fn start_ws_server() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:9001").await
        .map_err(|e| NodeError::IoError(e))?;
    info!("Listening on ws://127.0.0.1:9001");
    
    // Create our matchmaker
    let (matchmaker, mut matchmaker_rx) = create_matchmaker();
    
    // Store active connections
    let connections = NodeConnections::new(TokioMutex::new(HashMap::new()));
    
    // Spawn a task to handle matchmaker events
    let connections_for_events = connections.clone();
    tokio::spawn(async move {
        while let Ok(msg) = matchmaker_rx.recv().await {
            match msg {
                MatchmakerMessage::JobStatusUpdate(job_id, status) => {
                    // Broadcast job status updates to relevant nodes
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
            if let Err(e) = handle_connection(stream, peer, matchmaker_clone, connections_clone).await {
                error!("Error processing connection: {:?}", e);
            }
        });
    }
    
    Ok(())
}

async fn handle_connection(
    stream: TcpStream, 
    peer_id: String,
    matchmaker: SharedMatchMaker,
    connections: NodeConnections
) -> Result<(), Box<dyn std::error::Error>> {
    // Accept the WebSocket connection
    let ws_stream = accept_async(stream).await?;
    info!("WebSocket connection established with: {}", peer_id);
    
    // Split the WebSocket into sender and receiver
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Create a channel for sending messages to this specific connection
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(32);
    
    // Store the sender in our connections map
    {
        let mut connections = connections.lock().await;
        connections.insert(peer_id.clone(), tx);
    }
    
    // Forward messages from the channel to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            ws_sender.send(msg).await?;
        }
        Ok::<_, Box<dyn std::error::Error + Send>>(())
    });
    
    // Process incoming WebSocket messages
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
                            // Mark node as unavailable in the matchmaker
                            {
                                let mut mm = matchmaker.lock().unwrap();
                                mm.remove_node(&peer_id);
                            }
                            break;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    info!("WebSocket error: {:?}", e);
                    break;
                }
            }
        }
        
        // Clean up when connection closes
        {
            let mut connections = connections.lock().await;
            connections.remove(&peer_id);
            
            // Also mark node as unavailable in matchmaker
            let mut mm = matchmaker.lock().unwrap();
            mm.remove_node(&peer_id);
        }
        
        Ok::<_, Box<dyn std::error::Error + Send>>(())
    });
    
    // Wait for either task to complete
    tokio::select! {
        result = &mut send_task => {
            if let Err(e) = result? {
                info!("Error in send task: {:?}", e);
            }
        }
        result = &mut recv_task => {
            if let Err(e) = result? {
                info!("Error in receive task: {:?}", e);
            }
        }
    }
    
    Ok(())
}

async fn process_message(
    text: &str,
    peer_id: &str,
    matchmaker: &SharedMatchMaker
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the JSON message
    let parsed: Value = serde_json::from_str(text)?;
    
    // Extract message type if available
    let msg_type = parsed.get("type").and_then(Value::as_str);
    
    match msg_type {
        // Handle node stats/availability updates
        Some("stats") | None => {
            // For compatibility with existing code which may not specify a type
            if let (Some(cpu), Some(mem)) = (
                parsed.get("cpu").and_then(Value::as_f64),
                parsed.get("mem").and_then(Value::as_f64)
            ) {
                let node_id = parsed.get("node_id")
                    .and_then(Value::as_str)
                    .unwrap_or(peer_id)
                    .to_string();
                
                // Calculate available resources (simplified)
                let cpu_available = (100.0 - cpu) / 25.0; // Rough estimate: each 25% CPU = 1 core
                let mem_available = (100.0 - mem) * 40.96; // Rough estimate: 4GB total, convert percent to MB
                
                // Update the matchmaker with node capabilities
                let capabilities = NodeCapabilities {
                    node_id: node_id.clone(),
                    cpu_cores: cpu_available as f32,
                    memory_mb: mem_available as u64,
                    available: true,
                    reliability_score: 1.0, // Default for now
                    last_updated: Utc::now().timestamp() as u64,
                };
                
                {
                    let mut mm = matchmaker.lock().unwrap();
                    mm.update_node(capabilities);
                }
                
                info!("Updated node stats for {}: CPU={:.1}%, MEM={:.1}%", 
                    node_id, cpu, mem);
            }
        }
        
        // Handle explicit availability updates
        Some("availability_update") => {
            if let (Some(cpu_avail), Some(mem_avail), Some(status)) = (
                parsed.get("cpu_available").and_then(Value::as_f64),
                parsed.get("mem_available").and_then(Value::as_f64),
                parsed.get("status").and_then(Value::as_str)
            ) {
                let node_id = parsed.get("node_id")
                    .and_then(Value::as_str)
                    .unwrap_or(peer_id)
                    .to_string();
                
                let capabilities = NodeCapabilities {
                    node_id: node_id.clone(),
                    cpu_cores: cpu_avail as f32,
                    memory_mb: mem_avail as u64,
                    available: status == "ready",
                    reliability_score: parsed.get("reliability_score")
                        .and_then(Value::as_f64)
                        .unwrap_or(1.0) as f32,
                    last_updated: Utc::now().timestamp() as u64,
                };
                
                {
                    let mut mm = matchmaker.lock().unwrap();
                    mm.update_node(capabilities);
                }
                
                info!("Explicit availability update for {}: Status={}, CPU={:.1} cores, MEM={} MB", 
                    node_id, status, cpu_avail, mem_avail);
                update_node_availability(
                    &node_id, cpu_avail, mem_avail as u32, status
                ).await?;

            }
        }
        
        // Handle job status updates from nodes
        Some("job_status") => {
            if let (Some(job_id), Some(status)) = (
                parsed.get("job_id").and_then(Value::as_u64),
                parsed.get("status").and_then(Value::as_str)
            ) {
                let job_status = match status {
                    "queued" => JobStatus::Queued,
                    "matching" => JobStatus::Matching,
                    "assigned" => JobStatus::Assigned,
                    "running" => JobStatus::Running,
                    "completed" => JobStatus::Completed,
                    "failed" => JobStatus::Failed,
                    _ => JobStatus::Queued,
                };
                
                {
                    let mut mm = matchmaker.lock().unwrap();
                    mm.update_job_status(job_id, job_status);
                }
                
                info!("Job {} status updated to {}", job_id, status);
            }
        }
        
        // Handle job submissions
        Some("job_submit") => {
            if let Some(requirements) = parsed.get("requirements") {
                // Extract job requirements
                let cpu_cores = requirements.get("cpu_cores")
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0) as f32;
                
                let memory_mb = requirements.get("memory_mb")
                    .and_then(Value::as_u64)
                    .unwrap_or(1024);
                
                let expected_duration = requirements.get("expected_duration_sec")
                    .and_then(Value::as_u64)
                    .unwrap_or(3600);
                
                let priority = requirements.get("priority")
                    .and_then(Value::as_u64)
                    .unwrap_or(1) as u8;
                
                // Create job requirements
                let job_reqs = JobRequirements {
                    job_id: 0, // Will be assigned by matchmaker
                    cpu_cores,
                    memory_mb,
                    expected_duration_sec: expected_duration,
                    priority,
                };
                
                // Create the job
                let job = Job {
                    id: 0, // Will be assigned by matchmaker
                    requirements: job_reqs,
                    status: JobStatus::Queued,
                    assigned_node: None,
                    submitted_at: Utc::now().timestamp() as u64,
                    started_at: None,
                    completed_at: None,
                };
                
                // Submit to matchmaker
                let job_id = {
                    let mut mm = matchmaker.lock().unwrap();
                    mm.submit_job(job)
                };
                
                info!("Job submitted with ID: {}", job_id);
            }
        }
        
        _ => {
            info!("Unrecognized message type: {:?}", msg_type);
        }
    }
    
    Ok(())
}

async fn handle_job_status_update(
    job_id: u64,
    status: JobStatus,
    connections: &NodeConnections
) {
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
    let update_msg = json!({
        "type": "job_update",
        "job_id": job_id,
        "status": status_str,
        "timestamp": Utc::now().timestamp()
    });
    
    // Convert to websocket message
    let ws_msg = Message::Text(update_msg.to_string());
    
    // Send to all connected nodes (in a real system, we'd filter by relevance)
    let connections = connections.lock().await;
    for (_, tx) in connections.iter() {
        let _ = tx.send(ws_msg.clone()).await;
    }
}
