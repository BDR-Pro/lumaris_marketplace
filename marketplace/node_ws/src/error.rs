use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tungstenite::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    
    #[error("HTTP client error: {0}")]
    HttpClientError(String),
    
    #[error("Authentication error: {0}")]
    AuthError(String),
    
    #[error("Job scheduler error: {0}")]
    JobSchedulerError(String),
    
    #[error("VM manager error: {0}")]
    VmManagerError(String),
    
    #[error("Matchmaker error: {0}")]
    MatchmakerError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, NodeError>;

