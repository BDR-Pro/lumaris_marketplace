// File: marketplace/node_ws/src/config.rs

use std::fs::File;
use std::io::Read;
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::error::{Result, NodeError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeWsConfig {
    pub host: String,
    pub port: u16,
    pub admin_api_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestApiConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: String,
    pub console: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node_ws: NodeWsConfig,
    pub rest_api: RestApiConfig,
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node_ws: NodeWsConfig {
                host: "127.0.0.1".to_string(),
                port: 9001,
                admin_api_url: "http://127.0.0.1:8000".to_string(),
            },
            rest_api: RestApiConfig {
                host: "127.0.0.1".to_string(),
                port: 9002,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: "lumaris.log".to_string(),
                console: true,
            },
        }
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        
        // If the file doesn't exist, return the default config
        if !path.exists() {
            return Ok(Self::default());
        }
        
        // Open the file
        let mut file = File::open(path)
            .map_err(|e| NodeError::IoError(e))?;
        
        // Read the file contents
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| NodeError::IoError(e))?;
        
        // Parse the TOML
        let config: toml::Value = toml::from_str(&contents)
            .map_err(|e| NodeError::Unknown(format!("Failed to parse config: {}", e)))?;
        
        // Extract the node_ws section
        let node_ws = config.get("node_ws")
            .ok_or_else(|| NodeError::Unknown("Missing node_ws section in config".to_string()))?;
        
        let node_ws_config = NodeWsConfig {
            host: node_ws.get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("127.0.0.1")
                .to_string(),
            port: node_ws.get("port")
                .and_then(|v| v.as_integer())
                .unwrap_or(9001) as u16,
            admin_api_url: node_ws.get("admin_api_url")
                .and_then(|v| v.as_str())
                .unwrap_or("http://127.0.0.1:8000")
                .to_string(),
        };
        
        // Extract the rest_api section
        let rest_api = config.get("rest_api")
            .ok_or_else(|| NodeError::Unknown("Missing rest_api section in config".to_string()))?;
        
        let rest_api_config = RestApiConfig {
            host: rest_api.get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("127.0.0.1")
                .to_string(),
            port: rest_api.get("port")
                .and_then(|v| v.as_integer())
                .unwrap_or(9002) as u16,
        };
        
        // Extract the logging section
        let logging = config.get("logging")
            .ok_or_else(|| NodeError::Unknown("Missing logging section in config".to_string()))?;
        
        let logging_config = LoggingConfig {
            level: logging.get("level")
                .and_then(|v| v.as_str())
                .unwrap_or("info")
                .to_string(),
            file: logging.get("file")
                .and_then(|v| v.as_str())
                .unwrap_or("lumaris.log")
                .to_string(),
            console: logging.get("console")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        };
        
        Ok(Self {
            node_ws: node_ws_config,
            rest_api: rest_api_config,
            logging: logging_config,
        })
    }
}

