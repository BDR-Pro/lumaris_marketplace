use log::{info, error, warn, debug};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// Re-export the existing functions for backward compatibility
mod legacy;
pub use legacy::*;

// Include tests
#[cfg(test)]
mod tests;

// User role enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserRole {
    Buyer,
    Seller,
}

// API error types
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Authentication error: {0}")]
    AuthError(String),
    
    #[error("API error: {status} - {message}")]
    ApiResponseError { status: u16, message: String },
    
    #[error("Rate limit exceeded, retry after {retry_after} seconds")]
    RateLimitExceeded { retry_after: u64 },
    
    #[error("Request timeout")]
    Timeout,
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub request_id: Option<String>,
}

// Node registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistrationRequest {
    pub node_id: String,
    pub seller_id: String,
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub location: Option<String>,
    pub tags: Option<Vec<String>>,
}

// Node registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistrationResponse {
    pub node_id: String,
    pub registered: bool,
    pub token: Option<String>,
}

// Node availability update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAvailabilityRequest {
    pub node_id: String,
    pub available: bool,
    pub cpu_available: Option<f32>,
    pub memory_available: Option<u64>,
}

// VM creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmCreationRequest {
    pub buyer_id: String,
    pub vcpu_count: u32,
    pub mem_size_mib: u32,
    pub disk_size_gib: Option<u32>,
    pub image_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

// VM creation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmCreationResponse {
    pub vm_id: String,
    pub job_id: u64,
    pub status: String,
}

// VM status update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStatusUpdateRequest {
    pub vm_id: String,
    pub status: String,
    pub progress: Option<f32>,
    pub error: Option<String>,
}

// Buyer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyerStatistics {
    pub buyer_id: String,
    pub active_vms: u32,
    pub total_vms: u32,
    pub total_spend: f64,
    pub vms: Vec<VmInfo>,
}

// VM information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub vm_id: String,
    pub status: String,
    pub vcpu_count: u32,
    pub mem_size_mib: u32,
    pub created_at: DateTime<Utc>,
    pub cost: f64,
}

// Seller statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellerStatistics {
    pub seller_id: String,
    pub active_nodes: u32,
    pub total_nodes: u32,
    pub active_vms: u32,
    pub total_vms_hosted: u32,
    pub total_earnings: f64,
    pub nodes: HashMap<String, NodeInfo>,
}

// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub status: String,
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub vm_count: u32,
    pub earnings: f64,
}

// Authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

// API client configuration
#[derive(Debug, Clone)]
pub struct ApiClientConfig {
    pub api_url: String,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for ApiClientConfig {
    fn default() -> Self {
        Self {
            api_url: "http://127.0.0.1:8000".to_string(),
            timeout_ms: 5000,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

// Lumaris API client
#[derive(Debug, Clone)]
pub struct LumarisApiClient {
    config: ApiClientConfig,
    client: Client,
    auth_token: Arc<Mutex<Option<AuthToken>>>,
    role: UserRole,
    user_id: String,
}

impl LumarisApiClient {
    // Create a new API client
    pub fn new(config: ApiClientConfig, role: UserRole, user_id: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_default();
        
        Self {
            config,
            client,
            auth_token: Arc::new(Mutex::new(None)),
            role,
            user_id,
        }
    }
    
    // Get the base URL for API requests
    fn base_url(&self) -> String {
        self.config.api_url.clone()
    }
    
    // Get the authentication token
    async fn get_auth_token(&self) -> Option<String> {
        let token = self.auth_token.lock().await;
        token.as_ref().map(|t| t.token.clone())
    }
    
    // Set the authentication token
    async fn set_auth_token(&self, token: String, expires_at: DateTime<Utc>) {
        let mut auth_token = self.auth_token.lock().await;
        *auth_token = Some(AuthToken { token, expires_at });
    }
    
    // Check if the token is expired
    async fn is_token_expired(&self) -> bool {
        let token = self.auth_token.lock().await;
        match &*token {
            Some(t) => t.expires_at <= Utc::now(),
            None => true,
        }
    }
    
    // Authenticate with the API
    pub async fn authenticate(&self) -> Result<(), ApiError> {
        // Skip if we already have a valid token
        if !self.is_token_expired().await {
            return Ok(());
        }
        
        let url = format!("{}/auth/token", self.base_url());
        
        let payload = match self.role {
            UserRole::Buyer => json!({
                "role": "buyer",
                "buyer_id": self.user_id,
            }),
            UserRole::Seller => json!({
                "role": "seller",
                "seller_id": self.user_id,
            }),
        };
        
        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;
        
        match response.status() {
            StatusCode::OK => {
                let data: Value = response.json().await?;
                
                if let (Some(token), Some(expires_str)) = (
                    data.get("token").and_then(|t| t.as_str()),
                    data.get("expires_at").and_then(|e| e.as_str())
                ) {
                    if let Ok(expires_at) = expires_str.parse::<DateTime<Utc>>() {
                        self.set_auth_token(token.to_string(), expires_at).await;
                        info!("Successfully authenticated with the API");
                        return Ok(());
                    }
                }
                
                Err(ApiError::AuthError("Invalid token response".to_string()))
            },
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(ApiError::ApiResponseError {
                    status: status.as_u16(),
                    message: error_text,
                })
            }
        }
    }
    
    // Make an authenticated API request with retries
    async fn request<T: for<'de> Deserialize<'de>, U: Serialize>(
        &self,
        method: reqwest::Method,
        path: &str,
        payload: Option<&U>,
    ) -> Result<T, ApiError> {
        // Ensure we have a valid token
        self.authenticate().await?;
        
        let url = format!("{}{}", self.base_url(), path);
        let token = self.get_auth_token().await;
        
        for retry in 0..self.config.max_retries {
            let mut request = self.client.request(method.clone(), &url);
            
            // Add authentication if available
            if let Some(token) = &token {
                request = request.header("Authorization", format!("Bearer {}", token));
            }
            
            // Add payload if provided
            if let Some(data) = payload {
                request = request.json(data);
            }
            
            // Send the request
            match request.send().await {
                Ok(response) => {
                    match response.status() {
                        StatusCode::OK | StatusCode::CREATED => {
                            // Parse the response
                            match response.json::<T>().await {
                                Ok(data) => return Ok(data),
                                Err(e) => {
                                    error!("Failed to parse API response: {}", e);
                                    return Err(ApiError::SerializationError(e.into()));
                                }
                            }
                        },
                        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                            // Clear the token and retry authentication
                            {
                                let mut token = self.auth_token.lock().await;
                                *token = None;
                            }
                            
                            // Try to authenticate again
                            if let Err(e) = self.authenticate().await {
                                error!("Failed to re-authenticate: {}", e);
                                return Err(e);
                            }
                            
                            // Retry with the new token
                            continue;
                        },
                        StatusCode::TOO_MANY_REQUESTS => {
                            // Handle rate limiting
                            let retry_after = response.headers()
                                .get("Retry-After")
                                .and_then(|h| h.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(self.config.retry_delay_ms / 1000);
                            
                            warn!("Rate limit exceeded, retrying after {} seconds", retry_after);
                            
                            if retry < self.config.max_retries - 1 {
                                tokio::time::sleep(Duration::from_secs(retry_after)).await;
                                continue;
                            } else {
                                return Err(ApiError::RateLimitExceeded { retry_after });
                            }
                        },
                        status => {
                            let error_text = response.text().await.unwrap_or_default();
                            error!("API error: {} - {}", status, error_text);
                            
                            // Retry server errors
                            if status.is_server_error() && retry < self.config.max_retries - 1 {
                                let delay = self.config.retry_delay_ms * 2u64.pow(retry);
                                warn!("Server error, retrying in {} ms", delay);
                                tokio::time::sleep(Duration::from_millis(delay)).await;
                                continue;
                            }
                            
                            return Err(ApiError::ApiResponseError {
                                status: status.as_u16(),
                                message: error_text,
                            });
                        }
                    }
                },
                Err(e) => {
                    if e.is_timeout() {
                        warn!("Request timeout, retrying...");
                        
                        if retry < self.config.max_retries - 1 {
                            let delay = self.config.retry_delay_ms * 2u64.pow(retry);
                            tokio::time::sleep(Duration::from_millis(delay)).await;
                            continue;
                        } else {
                            return Err(ApiError::Timeout);
                        }
                    } else if e.is_connect() && retry < self.config.max_retries - 1 {
                        warn!("Connection error, retrying...");
                        let delay = self.config.retry_delay_ms * 2u64.pow(retry);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    } else {
                        error!("Network error: {}", e);
                        return Err(ApiError::NetworkError(e));
                    }
                }
            }
        }
        
        Err(ApiError::Unknown("Maximum retries exceeded".to_string()))
    }
    
    // Register a node (seller only)
    pub async fn register_node(
        &self,
        node_id: &str,
        cpu_cores: f32,
        memory_mb: u64,
        location: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<NodeRegistrationResponse, ApiError> {
        if self.role != UserRole::Seller {
            return Err(ApiError::AuthError("Only sellers can register nodes".to_string()));
        }
        
        let request = NodeRegistrationRequest {
            node_id: node_id.to_string(),
            seller_id: self.user_id.clone(),
            cpu_cores,
            memory_mb,
            location,
            tags,
        };
        
        let response: ApiResponse<NodeRegistrationResponse> = self.request(
            reqwest::Method::POST,
            "/nodes/register",
            Some(&request),
        ).await?;
        
        match response.data {
            Some(data) => Ok(data),
            None => Err(ApiError::ApiResponseError {
                status: 500,
                message: response.error.unwrap_or_else(|| "Unknown error".to_string()),
            }),
        }
    }
    
    // Update node availability (seller only)
    pub async fn update_node_availability(
        &self,
        node_id: &str,
        available: bool,
        cpu_available: Option<f32>,
        memory_available: Option<u64>,
    ) -> Result<bool, ApiError> {
        if self.role != UserRole::Seller {
            return Err(ApiError::AuthError("Only sellers can update node availability".to_string()));
        }
        
        let request = NodeAvailabilityRequest {
            node_id: node_id.to_string(),
            available,
            cpu_available,
            memory_available,
        };
        
        let response: ApiResponse<Value> = self.request(
            reqwest::Method::POST,
            &format!("/nodes/{}/availability", node_id),
            Some(&request),
        ).await?;
        
        Ok(response.success)
    }
    
    // Create a VM (buyer only)
    pub async fn create_vm(
        &self,
        vcpu_count: u32,
        mem_size_mib: u32,
        disk_size_gib: Option<u32>,
        image_id: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<VmCreationResponse, ApiError> {
        if self.role != UserRole::Buyer {
            return Err(ApiError::AuthError("Only buyers can create VMs".to_string()));
        }
        
        let request = VmCreationRequest {
            buyer_id: self.user_id.clone(),
            vcpu_count,
            mem_size_mib,
            disk_size_gib,
            image_id,
            tags,
        };
        
        let response: ApiResponse<VmCreationResponse> = self.request(
            reqwest::Method::POST,
            "/vms/create",
            Some(&request),
        ).await?;
        
        match response.data {
            Some(data) => Ok(data),
            None => Err(ApiError::ApiResponseError {
                status: 500,
                message: response.error.unwrap_or_else(|| "Unknown error".to_string()),
            }),
        }
    }
    
    // Update VM status (seller only)
    pub async fn update_vm_status(
        &self,
        vm_id: &str,
        status: &str,
        progress: Option<f32>,
        error: Option<String>,
    ) -> Result<bool, ApiError> {
        if self.role != UserRole::Seller {
            return Err(ApiError::AuthError("Only sellers can update VM status".to_string()));
        }
        
        let request = VmStatusUpdateRequest {
            vm_id: vm_id.to_string(),
            status: status.to_string(),
            progress,
            error,
        };
        
        let response: ApiResponse<Value> = self.request(
            reqwest::Method::POST,
            &format!("/vms/{}/status", vm_id),
            Some(&request),
        ).await?;
        
        Ok(response.success)
    }
    
    // Get buyer statistics (buyer only)
    pub async fn get_buyer_stats(&self) -> Result<BuyerStatistics, ApiError> {
        if self.role != UserRole::Buyer {
            return Err(ApiError::AuthError("Only buyers can get buyer statistics".to_string()));
        }
        
        let response: ApiResponse<BuyerStatistics> = self.request(
            reqwest::Method::GET,
            &format!("/buyers/{}/stats", self.user_id),
            None::<&()>,
        ).await?;
        
        match response.data {
            Some(data) => Ok(data),
            None => Err(ApiError::ApiResponseError {
                status: 500,
                message: response.error.unwrap_or_else(|| "Unknown error".to_string()),
            }),
        }
    }
    
    // Get seller statistics (seller only)
    pub async fn get_seller_stats(&self) -> Result<SellerStatistics, ApiError> {
        if self.role != UserRole::Seller {
            return Err(ApiError::AuthError("Only sellers can get seller statistics".to_string()));
        }
        
        let response: ApiResponse<SellerStatistics> = self.request(
            reqwest::Method::GET,
            &format!("/sellers/{}/stats", self.user_id),
            None::<&()>,
        ).await?;
        
        match response.data {
            Some(data) => Ok(data),
            None => Err(ApiError::ApiResponseError {
                status: 500,
                message: response.error.unwrap_or_else(|| "Unknown error".to_string()),
            }),
        }
    }
    
    // Terminate a VM (buyer only)
    pub async fn terminate_vm(&self, vm_id: &str) -> Result<bool, ApiError> {
        if self.role != UserRole::Buyer {
            return Err(ApiError::AuthError("Only buyers can terminate VMs".to_string()));
        }
        
        let response: ApiResponse<Value> = self.request(
            reqwest::Method::POST,
            &format!("/vms/{}/terminate", vm_id),
            None::<&()>,
        ).await?;
        
        Ok(response.success)
    }
    
    // Get health status of the API
    pub async fn health_check(&self) -> Result<bool, ApiError> {
        let url = format!("{}/health", self.base_url());
        
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => Err(ApiError::NetworkError(e)),
        }
    }
}
