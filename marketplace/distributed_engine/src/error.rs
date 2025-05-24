use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Job execution failed: {0}")]
    ExecutionError(String),
    
    #[error("Job not found: {0}")]
    JobNotFound(u64),
    
    #[error("Invalid job configuration: {0}")]
    InvalidJobConfig(String),
    
    #[error("Chunk processing error: {0}")]
    ChunkError(String),
    
    #[error("Result aggregation failed: {0}")]
    AggregationError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;

