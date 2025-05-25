use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use url::Url;
use log::{info, error, warn, debug};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;
use thiserror::Error;

use crate::api_client::UserRole;

// Include tests
#[cfg(test)]
mod tests;

// WebSocket error types
#[derive(Error, Debug)]
pub enum WsError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Message send error: {0}")]
    SendError(String),
    
    #[error("Message receive error: {0}")]
    ReceiveError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    
    #[error("Connection closed")]
    ConnectionClosed,
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

// WebSocket message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub type_: String,
    pub request_id: String,
    pub role: String,
    pub data: Value,
    pub timestamp: String,
}

impl WsMessage {
    // Create a new WebSocket message
    pub fn new(type_: &str, role: &UserRole, data: Value) -> Self {
        Self {
            type_: type_.to_string(),
            request_id: Uuid::new_v4().to_string(),
            role: match role {
                UserRole::Buyer => "buyer".to_string(),
                UserRole::Seller => "seller".to_string(),
            },
            data,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
    
    // Serialize to JSON
    pub fn to_json(&self) -> Result<String, WsError> {
        let mut value = serde_json::to_value(self)?;
        
        // Rename type_ to type for JSON
        if let Some(obj) = value.as_object_mut() {
            if let Some(type_value) = obj.remove("type_") {
                obj.insert("type".to_string(), type_value);
            }
        }
        
        Ok(serde_json::to_string(&value)?)
    }
    
    // Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, WsError> {
        let mut value: Value = serde_json::from_str(json)?;
        
        // Rename type to type_ for Rust
        if let Some(obj) = value.as_object_mut() {
            if let Some(type_value) = obj.remove("type") {
                obj.insert("type_".to_string(), type_value);
            }
        }
        
        Ok(serde_json::from_value(value)?)
    }
}

// WebSocket client configuration
#[derive(Debug, Clone)]
pub struct WsClientConfig {
    pub ws_url: String,
    pub reconnect_delay_ms: u64,
    pub max_reconnect_attempts: u32,
    pub heartbeat_interval_ms: u64,
    pub message_timeout_ms: u64,
}

impl Default for WsClientConfig {
    fn default() -> Self {
        Self {
            ws_url: "ws://127.0.0.1:3030/ws".to_string(),
            reconnect_delay_ms: 1000,
            max_reconnect_attempts: 10,
            heartbeat_interval_ms: 30000,
            message_timeout_ms: 5000,
        }
    }
}

// WebSocket client
#[derive(Debug)]
pub struct LumarisWsClient {
    config: WsClientConfig,
    role: UserRole,
    user_id: String,
    tx: mpsc::UnboundedSender<WsMessage>,
    rx: Arc<Mutex<mpsc::UnboundedReceiver<WsMessage>>>,
    connected: Arc<Mutex<bool>>,
    message_handlers: Arc<Mutex<Vec<Box<dyn Fn(WsMessage) -> bool + Send + Sync>>>>,
}

impl LumarisWsClient {
    // Create a new WebSocket client
    pub fn new(config: WsClientConfig, role: UserRole, user_id: String) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            config,
            role,
            user_id,
            tx,
            rx: Arc::new(Mutex::new(rx)),
            connected: Arc::new(Mutex::new(false)),
            message_handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    // Start the WebSocket client
    pub async fn start(&self) -> Result<(), WsError> {
        let config = self.config.clone();
        let role = self.role.clone();
        let user_id = self.user_id.clone();
        let rx = self.rx.clone();
        let connected = self.connected.clone();
        let message_handlers = self.message_handlers.clone();
        
        // Start the WebSocket connection in a separate task
        tokio::spawn(async move {
            let mut reconnect_attempts = 0;
            
            loop {
                match Self::connect_and_process(
                    &config,
                    &role,
                    &user_id,
                    rx.clone(),
                    connected.clone(),
                    message_handlers.clone(),
                ).await {
                    Ok(_) => {
                        // Connection closed normally
                        info!("WebSocket connection closed");
                        reconnect_attempts = 0;
                    },
                    Err(e) => {
                        // Connection error
                        error!("WebSocket error: {}", e);
                        
                        // Update connected status
                        let mut is_connected = connected.lock().await;
                        *is_connected = false;
                        
                        // Reconnect with exponential backoff
                        reconnect_attempts += 1;
                        
                        if reconnect_attempts <= config.max_reconnect_attempts {
                            let delay = config.reconnect_delay_ms * 2u64.pow(reconnect_attempts - 1);
                            warn!("Reconnecting in {} ms (attempt {}/{})", 
                                delay, reconnect_attempts, config.max_reconnect_attempts);
                            sleep(Duration::from_millis(delay)).await;
                        } else {
                            error!("Maximum reconnect attempts reached, giving up");
                            break;
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    // Connect to the WebSocket server and process messages
    async fn connect_and_process(
        config: &WsClientConfig,
        role: &UserRole,
        user_id: &str,
        rx: Arc<Mutex<mpsc::UnboundedReceiver<WsMessage>>>,
        connected: Arc<Mutex<bool>>,
        message_handlers: Arc<Mutex<Vec<Box<dyn Fn(WsMessage) -> bool + Send + Sync>>>>,
    ) -> Result<(), WsError> {
        // Parse the WebSocket URL
        let url = Url::parse(&config.ws_url)?;
        
        // Connect to the WebSocket server
        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| WsError::ConnectionError(e.to_string()))?;
        
        info!("Connected to WebSocket server at {}", config.ws_url);
        
        // Update connected status
        {
            let mut is_connected = connected.lock().await;
            *is_connected = true;
        }
        
        // Split the WebSocket stream
        let (mut write, mut read) = ws_stream.split();
        
        // Send initial identification message
        let init_message = WsMessage::new(
            "identify",
            role,
            json!({
                "user_id": user_id,
                "role": match role {
                    UserRole::Buyer => "buyer",
                    UserRole::Seller => "seller",
                },
                "client_version": env!("CARGO_PKG_VERSION"),
            }),
        );
        
        write.send(Message::Text(init_message.to_json()?)).await
            .map_err(|e| WsError::SendError(e.to_string()))?;
        
        // Start heartbeat task
        let heartbeat_task = {
            let role = role.clone();
            let mut write = write.clone();
            
            tokio::spawn(async move {
                loop {
                    sleep(Duration::from_millis(config.heartbeat_interval_ms)).await;
                    
                    // Check if still connected
                    let is_connected = {
                        let connected = connected.lock().await;
                        *connected
                    };
                    
                    if !is_connected {
                        break;
                    }
                    
                    // Send heartbeat
                    let heartbeat = WsMessage::new(
                        "heartbeat",
                        &role,
                        json!({
                            "timestamp": Utc::now().to_rfc3339(),
                        }),
                    );
                    
                    if let Ok(json) = heartbeat.to_json() {
                        if let Err(e) = write.send(Message::Text(json)).await {
                            error!("Failed to send heartbeat: {}", e);
                            break;
                        }
                    }
                }
            })
        };
        
        // Start message sender task
        let sender_task = {
            let role = role.clone();
            let mut write = write.clone();
            let mut rx = rx.lock().await;
            
            tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    match message.to_json() {
                        Ok(json) => {
                            if let Err(e) = write.send(Message::Text(json)).await {
                                error!("Failed to send message: {}", e);
                                break;
                            }
                        },
                        Err(e) => {
                            error!("Failed to serialize message: {}", e);
                        }
                    }
                }
            })
        };
        
        // Process incoming messages
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Received message: {}", text);
                    
                    // Parse the message
                    match WsMessage::from_json(&text) {
                        Ok(message) => {
                            // Process the message with registered handlers
                            let handlers = message_handlers.lock().await;
                            let mut handled = false;
                            
                            for handler in handlers.iter() {
                                if handler(message.clone()) {
                                    handled = true;
                                    break;
                                }
                            }
                            
                            if !handled {
                                debug!("No handler for message type: {}", message.type_);
                            }
                        },
                        Err(e) => {
                            error!("Failed to parse message: {}", e);
                        }
                    }
                },
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by server");
                    break;
                },
                Ok(_) => {
                    // Ignore other message types
                },
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    return Err(WsError::ReceiveError(e.to_string()));
                }
            }
        }
        
        // Cancel tasks
        heartbeat_task.abort();
        sender_task.abort();
        
        // Update connected status
        {
            let mut is_connected = connected.lock().await;
            *is_connected = false;
        }
        
        Ok(())
    }
    
    // Send a message
    pub fn send_message(&self, type_: &str, data: Value) -> Result<(), WsError> {
        let message = WsMessage::new(type_, &self.role, data);
        
        self.tx.send(message)
            .map_err(|e| WsError::SendError(e.to_string()))?;
        
        Ok(())
    }
    
    // Register a message handler
    pub async fn register_handler<F>(&self, handler: F)
    where
        F: Fn(WsMessage) -> bool + Send + Sync + 'static,
    {
        let mut handlers = self.message_handlers.lock().await;
        handlers.push(Box::new(handler));
    }
    
    // Check if connected
    pub async fn is_connected(&self) -> bool {
        let connected = self.connected.lock().await;
        *connected
    }
    
    // Wait for connection
    pub async fn wait_for_connection(&self, timeout_ms: u64) -> Result<(), WsError> {
        let start = std::time::Instant::now();
        
        while !self.is_connected().await {
            if start.elapsed() > Duration::from_millis(timeout_ms) {
                return Err(WsError::Timeout);
            }
            
            sleep(Duration::from_millis(100)).await;
        }
        
        Ok(())
    }
    
    // Create a VM (buyer only)
    pub fn create_vm(
        &self,
        job_id: u64,
        vcpu_count: u32,
        mem_size_mib: u32,
    ) -> Result<(), WsError> {
        if self.role != UserRole::Buyer {
            return Err(WsError::Unknown("Only buyers can create VMs".to_string()));
        }
        
        self.send_message(
            "create_vm",
            json!({
                "job_id": job_id,
                "buyer_id": self.user_id,
                "vcpu_count": vcpu_count,
                "mem_size_mib": mem_size_mib,
            }),
        )
    }
    
    // Register a node (seller only)
    pub fn register_node(
        &self,
        cpu_cores: f32,
        memory_mb: u64,
    ) -> Result<(), WsError> {
        if self.role != UserRole::Seller {
            return Err(WsError::Unknown("Only sellers can register nodes".to_string()));
        }
        
        self.send_message(
            "node_registration",
            json!({
                "seller_id": self.user_id,
                "capabilities": {
                    "cpu_cores": cpu_cores,
                    "memory_mb": memory_mb,
                }
            }),
        )
    }
    
    // Get buyer statistics (buyer only)
    pub fn get_buyer_stats(&self) -> Result<(), WsError> {
        if self.role != UserRole::Buyer {
            return Err(WsError::Unknown("Only buyers can get buyer statistics".to_string()));
        }
        
        self.send_message(
            "get_buyer_stats",
            json!({
                "buyer_id": self.user_id,
            }),
        )
    }
    
    // Get seller statistics (seller only)
    pub fn get_seller_stats(&self) -> Result<(), WsError> {
        if self.role != UserRole::Seller {
            return Err(WsError::Unknown("Only sellers can get seller statistics".to_string()));
        }
        
        self.send_message(
            "get_seller_stats",
            json!({
                "seller_id": self.user_id,
            }),
        )
    }
    
    // Terminate a VM (buyer only)
    pub fn terminate_vm(&self, vm_id: &str) -> Result<(), WsError> {
        if self.role != UserRole::Buyer {
            return Err(WsError::Unknown("Only buyers can terminate VMs".to_string()));
        }
        
        self.send_message(
            "terminate_vm",
            json!({
                "vm_id": vm_id,
                "buyer_id": self.user_id,
            }),
        )
    }
    
    // Disconnect a node (seller only)
    pub fn disconnect_node(&self, node_id: &str) -> Result<(), WsError> {
        if self.role != UserRole::Seller {
            return Err(WsError::Unknown("Only sellers can disconnect nodes".to_string()));
        }
        
        self.send_message(
            "node_disconnection",
            json!({
                "node_id": node_id,
                "seller_id": self.user_id,
            }),
        )
    }
}
