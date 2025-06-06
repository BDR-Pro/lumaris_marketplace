use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use futures_util::{StreamExt, SinkExt};
use warp::ws::{Message, WebSocket};
use warp::Filter;
use uuid::Uuid;
use chrono::Utc;
use log::{info, error, debug};
use serde_json::{json, Value};
use colored::*;

use crate::api_client::{
    update_node_availability,
    update_job_status
};

use crate::matchmaker::{
    SharedMatchMaker,
    NodeCapabilities,
    JobStatus
};

use crate::vm_manager::{
    VmManager,
    VmStatus
};

use crate::buyer_stats::{
    BuyerStatsManager,
    format_buyer_stats
};

use crate::seller_stats::{
    SellerStatsManager,
    format_seller_stats
};

#[cfg(test)]
mod tests;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type WsResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Type alias for node connections
type NodeConnections = Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>>;

// Start the WebSocket server
pub async fn run_ws_server(
    addr: &str,
    api_url: &str,
    matchmaker: SharedMatchMaker,
    connections: &NodeConnections,
    buyer_stats_manager: BuyerStatsManager,
    seller_stats_manager: SellerStatsManager
) -> Result<()> {
    // Create the VM manager
    let vm_base_path = std::env::var("VM_BASE_PATH").unwrap_or_else(|_| "/tmp/lumaris/vms".to_string());
    let vm_manager = VmManager::new(&vm_base_path, api_url);
    
    // Create the WebSocket handler
    let ws_route = create_ws_handler(api_url, matchmaker.clone(), connections.clone(), vm_manager, buyer_stats_manager, seller_stats_manager);
    
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

// Handle WebSocket connections
async fn handle_websocket_connection(
    ws: WebSocket,
    api_url: String,
    matchmaker: SharedMatchMaker,
    _rx: broadcast::Receiver<String>,
    connections: NodeConnections,
    vm_manager: VmManager,
    buyer_stats_manager: BuyerStatsManager,
    seller_stats_manager: SellerStatsManager
) {
    // Split the WebSocket into a sender and receiver
    let (mut ws_sender, mut ws_receiver) = ws.split();
    
    // Generate a unique ID for this connection
    let peer_id = Uuid::new_v4().to_string();
    info!("New WebSocket connection: {}", peer_id);
    
    // Create a channel for sending messages to this WebSocket
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    
    // Store the sender in the connections map
    {
        let mut conns = connections.lock().await;
        conns.insert(peer_id.clone(), tx.clone());
    }
    
    // Create a channel for sending messages to the WebSocket
    let (ws_sender_tx, mut ws_sender_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    
    // Clone for use in the tasks
    let matchmaker_clone = matchmaker.clone();
    let peer_id_clone = peer_id.clone();
    let connections_clone = connections.clone();
    let ws_sender_tx_clone = ws_sender_tx.clone();
    let api_url_clone = api_url.clone();
    let vm_manager_clone = vm_manager.clone();
    let buyer_stats_manager_clone = buyer_stats_manager.clone();
    let seller_stats_manager_clone = seller_stats_manager.clone();
    
    // Task to forward messages from the channel to the WebSocket
    let ws_sender_task = tokio::spawn(async move {
        while let Some(message) = ws_sender_rx.recv().await {
            if let Err(e) = ws_sender.send(message).await {
                error!("Error sending message to WebSocket: {}", e);
                break;
            }
        }
    });
    
    // Handle incoming WebSocket messages
    let message_handler = tokio::spawn({
        let ws_sender_tx = ws_sender_tx_clone.clone();
        let api_url_clone = api_url_clone.clone();
        let vm_manager_clone = vm_manager_clone.clone();
        let buyer_stats_manager_clone = buyer_stats_manager_clone.clone();
        let seller_stats_manager_clone = seller_stats_manager_clone.clone();
        async move {
            while let Some(result) = ws_receiver.next().await {
                match result {
                    Ok(msg) => {
                        // Skip if not a text message
                        if !msg.is_text() {
                            continue;
                        }
                        
                        // Get the message text
                        let msg_text = msg.to_str().unwrap_or_default();
                        
                        // Process the message
                        if let Err(e) = process_message(
                            msg_text, 
                            &peer_id_clone, 
                            &api_url_clone, 
                            &matchmaker_clone, 
                            &connections_clone, 
                            &ws_sender_tx,
                            &vm_manager_clone,
                            &buyer_stats_manager_clone,
                            &seller_stats_manager_clone
                        ).await {
                            error!("Error processing message: {}", e);
                            
                            // Send error response
                            let error_response = json!({
                                "type": "error",
                                "message": format!("Error processing message: {}", e)
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
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
                let mut conns = connections_clone.lock().await;
                conns.remove(&peer_id_clone);
            }
            
            info!("WebSocket connection closed: {}", peer_id_clone);
        }
    });
    
    // Forward broadcast messages to this WebSocket
    let forward_task = tokio::spawn({
        let ws_sender_tx = ws_sender_tx_clone;
        async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = ws_sender_tx.send(Message::text(msg.clone())) {
                    error!("Error sending message: {}", e);
                    break;
                }
            }
        }
    });
    
    // Wait for all tasks to complete
    tokio::select! {
        _ = message_handler => {},
        _ = forward_task => {},
        _ = ws_sender_task => {},
    }
}

// Process incoming WebSocket messages
pub async fn process_message(
    message: &str,
    peer_id: &str,
    api_url: &str,
    matchmaker: &SharedMatchMaker,
    connections: &Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>>>,
    ws_sender_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    vm_manager: &VmManager,
    buyer_stats_manager: &BuyerStatsManager,
    seller_stats_manager: &SellerStatsManager
) -> WsResult<()> {
    // Parse the message as JSON
    let parsed: Value = serde_json::from_str(message)?;
    
    // Process the message based on its type
    if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
        match msg_type {
            "node_registration" => {
                info!("Node registration from: {}", peer_id);
                
                // Extract node capabilities
                if let Some(capabilities) = parsed.get("capabilities") {
                    let cpu_cores = capabilities.get("cpu_cores").and_then(|c| c.as_f64()).unwrap_or(0.0) as f32;
                    let memory_mb = capabilities.get("memory_mb").and_then(|m| m.as_u64()).unwrap_or(0);
                    
                    info!("{} {} {} {} {} {}", 
                        "Node".green(), 
                        peer_id.bright_yellow(), 
                        "registered with".green(), 
                        cpu_cores.to_string().bright_white(), 
                        "CPU cores and".green(), 
                        format!("{} MB", memory_mb).bright_white()
                    );
                    
                    // Register node with matchmaker
                    let node_capabilities = NodeCapabilities {
                        node_id: peer_id.to_string(),
                        cpu_cores,
                        memory_mb,
                    };
                    
                    let mut mm = matchmaker.lock().await;
                    mm.register_node(node_capabilities);
                    
                    // Extract seller ID if provided
                    if let Some(seller_id) = parsed.get("seller_id").and_then(|s| s.as_str()) {
                        // Record node registration in seller stats
                        seller_stats_manager.record_node_registration(seller_id, peer_id, cpu_cores, memory_mb).await;
                    }
                    
                    // Send registration confirmation
                    let response = json!({
                        "type": "node_registered",
                        "node_id": peer_id,
                        "status": "registered"
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                        error!("Error sending registration confirmation: {}", e);
                    }
                    
                    // Update node availability in API
                    if let Err(e) = update_node_availability(api_url, peer_id, true).await {
                        error!("Failed to update node availability in API: {}", e);
                    }
                }
            },
            "node_status" => {
                // Update node status
                let available = parsed["available"].as_bool().unwrap_or(true);
                
                {
                    let mut mm = matchmaker.lock().await;
                    mm.update_node_availability(peer_id.to_string(), available);
                }
                
                // Update node availability in the API
                let node_id_copy = peer_id.to_string();
                let api_url_copy = api_url.to_string();
                tokio::spawn(async move {
                    if let Err(e) = update_node_availability(&api_url_copy, &node_id_copy, available).await {
                        error!("Failed to update node availability in API: {}", e);
                    }
                });
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
                        let mut mm = matchmaker.lock().await;
                        let _ = mm.update_job_status(job_id, status_str.to_string());
                    }
                    
                    // Update job status in the API
                    let job_id_copy = job_id;
                    let status_str_copy = status_str.to_string();
                    let api_url_copy = api_url.to_string();
                    tokio::spawn(async move {
                        if let Err(e) = update_job_status(&api_url_copy, job_id_copy, &status_str_copy, result_data).await {
                            error!("Failed to update job status: {}", e);
                        }
                    });
                    
                    // Broadcast job status update to all connections
                    handle_job_status_update(job_id, status_str, connections).await?;
                }
            },
            "create_vm" => {
                // Extract VM creation parameters
                if let (Some(job_id), Some(buyer_id), Some(vcpu_count), Some(mem_size_mib)) = (
                    parsed.get("job_id").and_then(|j| j.as_u64()),
                    parsed.get("buyer_id").and_then(|b| b.as_str()),
                    parsed.get("vcpu_count").and_then(|v| v.as_u64()).map(|v| v as u32),
                    parsed.get("mem_size_mib").and_then(|m| m.as_u64()).map(|m| m as u32)
                ) {
                    info!("{} {} {}", "Creating VM for job".green(), job_id.to_string().bright_yellow(), format!("(buyer: {})", buyer_id).bright_cyan());
                    
                    // Extract seller ID if provided
                    let seller_id = parsed.get("seller_id").and_then(|s| s.as_str()).unwrap_or("unknown");
                    
                    // Create VM
                    match vm_manager.create_vm(job_id, buyer_id, vcpu_count, mem_size_mib).await {
                        Ok(vm_id) => {
                            info!("{} {}", "VM created:".green(), vm_id.bright_yellow());
                            
                            // Record VM creation in buyer stats
                            buyer_stats_manager.record_vm_creation(&vm_id, job_id, buyer_id, vcpu_count, mem_size_mib).await;
                            
                            // Record VM hosting in seller stats
                            if seller_id != "unknown" {
                                seller_stats_manager.record_vm_hosting(seller_id, peer_id, &vm_id, job_id, buyer_id, vcpu_count, mem_size_mib).await;
                            }
                            
                            // Send VM creation confirmation
                            let response = json!({
                                "type": "vm_created",
                                "job_id": job_id,
                                "buyer_id": buyer_id,
                                "vm_id": vm_id,
                                "status": "creating"
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                                error!("Error sending VM creation confirmation: {}", e);
                            }
                        },
                        Err(e) => {
                            error!("Failed to create VM: {}", e);
                            
                            // Send error response
                            let error_response = json!({
                                "type": "vm_creation_failed",
                                "job_id": job_id,
                                "buyer_id": buyer_id,
                                "error": format!("Failed to create VM: {}", e)
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                                error!("Error sending VM creation error: {}", e);
                            }
                        }
                    }
                } else {
                    // Send error response for missing parameters
                    let error_response = json!({
                        "type": "error",
                        "message": "Missing required parameters for VM creation"
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                        error!("Error sending parameter error: {}", e);
                    }
                }
            },
            "stop_vm" => {
                // Extract VM ID
                if let Some(vm_id) = parsed.get("vm_id").and_then(|v| v.as_str()) {
                    info!("{} {}", "Stopping VM:".yellow(), vm_id.bright_yellow());
                    
                    // Stop VM
                    match vm_manager.stop_vm(vm_id).await {
                        Ok(_) => {
                            info!("{} {}", "VM stopped:".yellow(), vm_id.bright_yellow());
                            
                            // Send VM stop confirmation
                            let response = json!({
                                "type": "vm_stopped",
                                "vm_id": vm_id,
                                "status": "stopped"
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                                error!("Error sending VM stop confirmation: {}", e);
                            }
                        },
                        Err(e) => {
                            error!("Failed to stop VM: {}", e);
                            
                            // Send error response
                            let error_response = json!({
                                "type": "vm_stop_failed",
                                "vm_id": vm_id,
                                "error": format!("Failed to stop VM: {}", e)
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                                error!("Error sending VM stop error: {}", e);
                            }
                        }
                    }
                } else {
                    // Send error response for missing VM ID
                    let error_response = json!({
                        "type": "error",
                        "message": "Missing VM ID for VM stop"
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                        error!("Error sending parameter error: {}", e);
                    }
                }
            },
            "terminate_vm" => {
                // Extract VM ID
                if let Some(vm_id) = parsed.get("vm_id").and_then(|v| v.as_str()) {
                    info!("{} {}", "Terminating VM:".yellow(), vm_id.bright_yellow());
                    
                    // Get VM info for buyer ID
                    let buyer_id = match vm_manager.get_vm(vm_id).await {
                        Ok(vm) => vm.config.buyer_id,
                        Err(_) => "unknown".to_string(),
                    };
                    
                    // Extract seller ID if provided
                    let seller_id = parsed.get("seller_id").and_then(|s| s.as_str()).unwrap_or("unknown");
                    
                    // Terminate VM
                    match vm_manager.terminate_vm(vm_id).await {
                        Ok(_) => {
                            info!("{} {}", "VM terminated:".yellow(), vm_id.bright_yellow());
                            
                            // Record VM termination in buyer stats
                            buyer_stats_manager.record_vm_termination(vm_id, &buyer_id).await;
                            
                            // Record VM termination in seller stats
                            if seller_id != "unknown" {
                                seller_stats_manager.record_vm_termination(seller_id, vm_id).await;
                            }
                            
                            // Send VM termination confirmation
                            let response = json!({
                                "type": "vm_terminated",
                                "vm_id": vm_id,
                                "status": "terminated"
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                                error!("Error sending VM termination confirmation: {}", e);
                            }
                        },
                        Err(e) => {
                            error!("Failed to terminate VM: {}", e);
                            
                            // Send error response
                            let error_response = json!({
                                "type": "vm_termination_failed",
                                "vm_id": vm_id,
                                "error": format!("Failed to terminate VM: {}", e)
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                                error!("Error sending VM termination error: {}", e);
                            }
                        }
                    }
                } else {
                    // Send error response for missing VM ID
                    let error_response = json!({
                        "type": "error",
                        "message": "Missing VM ID for VM termination"
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                        error!("Error sending parameter error: {}", e);
                    }
                }
            },
            "get_vm_status" => {
                // Extract VM ID
                if let Some(vm_id) = parsed.get("vm_id").and_then(|v| v.as_str()) {
                    info!("{} {}", "Getting VM status:".cyan(), vm_id.bright_cyan());
                    
                    // Get VM status
                    match vm_manager.get_vm(vm_id).await {
                        Ok(vm) => {
                            info!("{} {}", "VM status:".cyan(), vm.status);
                            
                            // Send VM status
                            let response = json!({
                                "type": "vm_status",
                                "vm_id": vm_id,
                                "job_id": vm.config.job_id,
                                "buyer_id": vm.config.buyer_id,
                                "status": format!("{:?}", vm.status),
                                "ip_address": vm.ip_address,
                                "created_at": vm.created_at,
                                "updated_at": vm.updated_at,
                                "error_message": vm.error_message
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                                error!("Error sending VM status: {}", e);
                            }
                        },
                        Err(e) => {
                            error!("Failed to get VM status: {}", e);
                            
                            // Send error response
                            let error_response = json!({
                                "type": "vm_status_failed",
                                "vm_id": vm_id,
                                "error": format!("Failed to get VM status: {}", e)
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                                error!("Error sending VM status error: {}", e);
                            }
                        }
                    }
                } else {
                    // Send error response for missing VM ID
                    let error_response = json!({
                        "type": "error",
                        "message": "Missing VM ID for VM status"
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                        error!("Error sending parameter error: {}", e);
                    }
                }
            },
            "get_buyer_vms" => {
                // Extract buyer ID
                if let Some(buyer_id) = parsed.get("buyer_id").and_then(|b| b.as_str()) {
                    info!("{} {}", "Getting VMs for buyer:".cyan(), buyer_id.bright_cyan());
                    
                    // Get buyer VMs
                    let vms = vm_manager.get_vms_by_buyer(buyer_id).await;
                    
                    // Convert VMs to JSON
                    let vm_list: Vec<Value> = vms.iter().map(|vm| {
                        json!({
                            "vm_id": vm.config.vm_id,
                            "job_id": vm.config.job_id,
                            "status": format!("{:?}", vm.status),
                            "ip_address": vm.ip_address,
                            "created_at": vm.created_at,
                            "updated_at": vm.updated_at,
                            "error_message": vm.error_message
                        })
                    }).collect();
                    
                    // Send VM list
                    let response = json!({
                        "type": "buyer_vms",
                        "buyer_id": buyer_id,
                        "vms": vm_list
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                        error!("Error sending buyer VMs: {}", e);
                    }
                } else {
                    // Send error response for missing buyer ID
                    let error_response = json!({
                        "type": "error",
                        "message": "Missing buyer ID for buyer VMs"
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                        error!("Error sending parameter error: {}", e);
                    }
                }
            },
            "get_buyer_stats" => {
                // Extract buyer ID
                if let Some(buyer_id) = parsed.get("buyer_id").and_then(|b| b.as_str()) {
                    info!("{} {}", "Getting stats for buyer:".cyan(), buyer_id.bright_cyan());
                    
                    // Get buyer stats
                    match buyer_stats_manager.get_buyer_stats(buyer_id).await {
                        Some(stats) => {
                            // Format buyer stats
                            let formatted_stats = format_buyer_stats(&stats);
                            
                            // Send buyer stats
                            let response = json!({
                                "type": "buyer_stats",
                                "buyer_id": buyer_id,
                                "stats": {
                                    "session_start": stats.session_start.to_rfc3339(),
                                    "active_vms": stats.active_vms,
                                    "total_vms_spawned": stats.total_vms_spawned,
                                    "total_earnings": stats.total_earnings,
                                    "countries": stats.countries,
                                    "formatted_stats": formatted_stats
                                }
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                                error!("Error sending buyer stats: {}", e);
                            }
                        },
                        None => {
                            // Send empty stats
                            let response = json!({
                                "type": "buyer_stats",
                                "buyer_id": buyer_id,
                                "stats": {
                                    "session_start": Utc::now().to_rfc3339(),
                                    "active_vms": 0,
                                    "total_vms_spawned": 0,
                                    "total_earnings": 0.0,
                                    "countries": {},
                                    "formatted_stats": format!("No stats available for buyer {}", buyer_id)
                                }
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                                error!("Error sending empty buyer stats: {}", e);
                            }
                        }
                    }
                } else {
                    // Send error response for missing buyer ID
                    let error_response = json!({
                        "type": "error",
                        "message": "Missing buyer ID for buyer stats"
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                        error!("Error sending parameter error: {}", e);
                    }
                }
            },
            "get_seller_stats" => {
                // Extract seller ID
                if let Some(seller_id) = parsed.get("seller_id").and_then(|s| s.as_str()) {
                    info!("{} {}", "Getting stats for seller:".cyan(), seller_id.bright_cyan());
                    
                    // Get seller stats
                    match seller_stats_manager.get_seller_stats(seller_id).await {
                        Some(stats) => {
                            // Format seller stats
                            let formatted_stats = format_seller_stats(&stats);
                            
                            // Send seller stats
                            let response = json!({
                                "type": "seller_stats",
                                "seller_id": seller_id,
                                "stats": {
                                    "session_start": stats.session_start.to_rfc3339(),
                                    "active_nodes": stats.active_nodes,
                                    "total_nodes": stats.total_nodes,
                                    "active_vms": stats.active_vms,
                                    "total_vms_hosted": stats.total_vms_hosted,
                                    "total_earnings": stats.total_earnings,
                                    "total_cpu_cores": stats.total_cpu_cores,
                                    "total_memory_mb": stats.total_memory_mb,
                                    "nodes": stats.nodes,
                                    "buyer_distribution": stats.buyer_distribution,
                                    "formatted_stats": formatted_stats
                                }
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                                error!("Error sending seller stats: {}", e);
                            }
                        },
                        None => {
                            // Send empty stats
                            let response = json!({
                                "type": "seller_stats",
                                "seller_id": seller_id,
                                "stats": {
                                    "session_start": Utc::now().to_rfc3339(),
                                    "active_nodes": 0,
                                    "total_nodes": 0,
                                    "active_vms": 0,
                                    "total_vms_hosted": 0,
                                    "total_earnings": 0.0,
                                    "total_cpu_cores": 0.0,
                                    "total_memory_mb": 0,
                                    "nodes": {},
                                    "buyer_distribution": {},
                                    "formatted_stats": format!("No stats available for seller {}", seller_id)
                                }
                            }).to_string();
                            
                            if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                                error!("Error sending empty seller stats: {}", e);
                            }
                        }
                    }
                } else {
                    // Send error response for missing seller ID
                    let error_response = json!({
                        "type": "error",
                        "message": "Missing seller ID for seller stats"
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(error_response)) {
                        error!("Error sending parameter error: {}", e);
                    }
                }
            },
            "node_disconnection" => {
                // Extract seller ID
                if let Some(seller_id) = parsed.get("seller_id").and_then(|s| s.as_str()) {
                    info!("{} {} {}", "Node".yellow(), peer_id.bright_yellow(), "disconnecting".yellow());
                    
                    // Record node disconnection in seller stats
                    seller_stats_manager.record_node_disconnection(seller_id, peer_id).await;
                    
                    // Update node availability in matchmaker
                    let mut mm = matchmaker.lock().await;
                    mm.set_node_availability(peer_id, false);
                    
                    // Update node availability in API
                    if let Err(e) = update_node_availability(api_url, peer_id, false).await {
                        error!("Failed to update node availability in API: {}", e);
                    }
                    
                    // Send disconnection confirmation
                    let response = json!({
                        "type": "node_disconnected",
                        "node_id": peer_id,
                        "status": "disconnected"
                    }).to_string();
                    
                    if let Err(e) = ws_sender_tx.send(Message::text(response)) {
                        error!("Error sending disconnection confirmation: {}", e);
                    }
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
    let conns = connections.lock().await;
    for (_, tx) in conns.iter() {
        if let Err(e) = tx.send(update_msg.clone()) {
            error!("Error broadcasting job status update: {}", e);
        }
    }
    
    Ok(())
}

// Create a WebSocket handler for the matchmaker
pub fn create_ws_handler(
    api_url: &str,
    matchmaker: SharedMatchMaker,
    connections: NodeConnections,
    vm_manager: VmManager,
    buyer_stats_manager: BuyerStatsManager,
    seller_stats_manager: SellerStatsManager
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Create a broadcast channel for sending messages to all connected clients
    let (tx, _rx) = broadcast::channel::<String>(100);
    
    // Clone the API URL for use in the handler
    let api_url = api_url.to_string();
    
    // Create the WebSocket route
    warp::path("ws")
        .and(warp::ws())
        .and(with_api_url(api_url))
        .and(with_matchmaker(matchmaker))
        .and(with_broadcaster(tx.clone()))
        .and(with_connections(connections))
        .and(with_vm_manager(vm_manager))
        .and(with_buyer_stats_manager(buyer_stats_manager))
        .and(with_seller_stats_manager(seller_stats_manager))
        .map(move |ws: warp::ws::Ws, api_url: String, matchmaker: SharedMatchMaker, tx: broadcast::Sender<String>, connections: NodeConnections, vm_manager: VmManager, buyer_stats_manager: BuyerStatsManager, seller_stats_manager: SellerStatsManager| {
            // Clone tx for the closure
            let tx_clone = tx.clone();
            
            ws.on_upgrade(move |socket| {
                // Create a new receiver for this connection
                let rx = tx_clone.subscribe();
                
                // Handle the WebSocket connection
                // Return a future that resolves to ()
                async move {
                    handle_websocket_connection(socket, api_url, matchmaker, rx, connections, vm_manager, buyer_stats_manager, seller_stats_manager).await;
                }
            })
        })
}

// Helper function to pass the API URL to the handler
fn with_api_url(api_url: String) -> impl Filter<Extract = (String,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || api_url.clone())
}

// Helper function to pass the matchmaker to the handler
fn with_matchmaker(matchmaker: SharedMatchMaker) -> impl Filter<Extract = (SharedMatchMaker,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || matchmaker.clone())
}

// Helper function to pass the broadcaster to the handler
fn with_broadcaster(tx: broadcast::Sender<String>) -> impl Filter<Extract = (broadcast::Sender<String>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || tx.clone())
}

// Helper function to pass the connections to the handler
fn with_connections(connections: NodeConnections) -> impl Filter<Extract = (NodeConnections,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || connections.clone())
}

// Helper function to pass the VM manager to the handler
fn with_vm_manager(vm_manager: VmManager) -> impl Filter<Extract = (VmManager,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || vm_manager.clone())
}

// Helper function to pass the buyer stats manager to the handler
fn with_buyer_stats_manager(buyer_stats_manager: BuyerStatsManager) -> impl Filter<Extract = (BuyerStatsManager,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || buyer_stats_manager.clone())
}

// Helper function to pass the seller stats manager to the handler
fn with_seller_stats_manager(seller_stats_manager: SellerStatsManager) -> impl Filter<Extract = (SellerStatsManager,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || seller_stats_manager.clone())
}
