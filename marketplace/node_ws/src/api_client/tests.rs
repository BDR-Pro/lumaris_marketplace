#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;
    use mockito::{mock, server_url};

    // Test API client configuration
    #[test]
    fn test_api_client_config_default() {
        let config = ApiClientConfig::default();
        assert_eq!(config.api_url, "http://127.0.0.1:8000");
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 1000);
    }

    // Test API client creation
    #[test]
    fn test_api_client_creation() {
        let config = ApiClientConfig::default();
        let client = LumarisApiClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        assert_eq!(client.role, UserRole::Buyer);
        assert_eq!(client.user_id, "test-buyer");
    }

    // Test API error creation
    #[test]
    fn test_api_error_creation() {
        let error = ApiError::AuthError("Invalid token".to_string());
        assert!(error.to_string().contains("Invalid token"));

        let error = ApiError::ApiResponseError {
            status: 404,
            message: "Not found".to_string(),
        };
        assert!(error.to_string().contains("404"));
        assert!(error.to_string().contains("Not found"));
    }

    // Test authentication
    #[tokio::test]
    async fn test_authentication_success() {
        let mock_server = mock("POST", "/auth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"token": "test-token", "expires_at": "2099-01-01T00:00:00Z"}"#)
            .create();

        let mut config = ApiClientConfig::default();
        config.api_url = server_url();

        let client = LumarisApiClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        let result = client.authenticate().await;
        
        assert!(result.is_ok());
        mock_server.assert();
    }

    // Test authentication failure
    #[tokio::test]
    async fn test_authentication_failure() {
        let mock_server = mock("POST", "/auth/token")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid credentials"}"#)
            .create();

        let mut config = ApiClientConfig::default();
        config.api_url = server_url();

        let client = LumarisApiClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        let result = client.authenticate().await;
        
        assert!(result.is_err());
        match result {
            Err(ApiError::ApiResponseError { status, .. }) => {
                assert_eq!(status, 401);
            },
            _ => panic!("Expected ApiResponseError"),
        }
        mock_server.assert();
    }

    // Test buyer operations
    #[tokio::test]
    async fn test_create_vm_buyer_only() {
        let mut config = ApiClientConfig::default();
        config.api_url = server_url();

        // Test with seller role (should fail)
        let seller_client = LumarisApiClient::new(config.clone(), UserRole::Seller, "test-seller".to_string());
        let result = seller_client.create_vm(2, 1024, None, None, None).await;
        
        assert!(result.is_err());
        match result {
            Err(ApiError::AuthError(msg)) => {
                assert!(msg.contains("Only buyers"));
            },
            _ => panic!("Expected AuthError"),
        }

        // Test with buyer role (mock success)
        let mock_server = mock("POST", "/vms/create")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": true, "data": {"vm_id": "test-vm", "job_id": 123, "status": "creating"}}"#)
            .create();

        // Mock authentication first
        let auth_mock = mock("POST", "/auth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"token": "test-token", "expires_at": "2099-01-01T00:00:00Z"}"#)
            .create();

        let buyer_client = LumarisApiClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        let result = buyer_client.create_vm(2, 1024, None, None, None).await;
        
        assert!(result.is_ok());
        if let Ok(response) = result {
            assert_eq!(response.vm_id, "test-vm");
            assert_eq!(response.job_id, 123);
            assert_eq!(response.status, "creating");
        }
        
        mock_server.assert();
        auth_mock.assert();
    }

    // Test seller operations
    #[tokio::test]
    async fn test_register_node_seller_only() {
        let mut config = ApiClientConfig::default();
        config.api_url = server_url();

        // Test with buyer role (should fail)
        let buyer_client = LumarisApiClient::new(config.clone(), UserRole::Buyer, "test-buyer".to_string());
        let result = buyer_client.register_node("test-node", 4.0, 8192, None, None).await;
        
        assert!(result.is_err());
        match result {
            Err(ApiError::AuthError(msg)) => {
                assert!(msg.contains("Only sellers"));
            },
            _ => panic!("Expected AuthError"),
        }

        // Test with seller role (mock success)
        let mock_server = mock("POST", "/nodes/register")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": true, "data": {"node_id": "test-node", "registered": true, "token": "node-token"}}"#)
            .create();

        // Mock authentication first
        let auth_mock = mock("POST", "/auth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"token": "test-token", "expires_at": "2099-01-01T00:00:00Z"}"#)
            .create();

        let seller_client = LumarisApiClient::new(config, UserRole::Seller, "test-seller".to_string());
        let result = seller_client.register_node("test-node", 4.0, 8192, None, None).await;
        
        assert!(result.is_ok());
        if let Ok(response) = result {
            assert_eq!(response.node_id, "test-node");
            assert_eq!(response.registered, true);
            assert_eq!(response.token, Some("node-token".to_string()));
        }
        
        mock_server.assert();
        auth_mock.assert();
    }

    // Test retry mechanism
    #[tokio::test]
    async fn test_retry_mechanism() {
        // First request fails with 500, second succeeds
        let first_mock = mock("POST", "/auth/token")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Server error"}"#)
            .create();

        let second_mock = mock("POST", "/auth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"token": "test-token", "expires_at": "2099-01-01T00:00:00Z"}"#)
            .create();

        let mut config = ApiClientConfig::default();
        config.api_url = server_url();
        config.retry_delay_ms = 100; // Fast retry for testing

        let client = LumarisApiClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        
        // Sleep to allow the first request to fail and retry
        sleep(Duration::from_millis(200)).await;
        
        let result = client.authenticate().await;
        assert!(result.is_ok());
        
        first_mock.assert();
        second_mock.assert();
    }

    // Test token expiration and refresh
    #[tokio::test]
    async fn test_token_expiration() {
        // First token expires soon
        let first_auth_mock = mock("POST", "/auth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"token": "expired-token", "expires_at": "2000-01-01T00:00:00Z"}"#)
            .create();

        // Second token is valid
        let second_auth_mock = mock("POST", "/auth/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"token": "valid-token", "expires_at": "2099-01-01T00:00:00Z"}"#)
            .create();

        // API call with valid token
        let api_mock = mock("GET", "/buyers/test-buyer/stats")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": true, "data": {"buyer_id": "test-buyer", "active_vms": 0, "total_vms": 0, "total_spend": 0.0, "vms": []}}"#)
            .create();

        let mut config = ApiClientConfig::default();
        config.api_url = server_url();

        let client = LumarisApiClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        
        // First authenticate to get the expired token
        let _ = client.authenticate().await;
        first_auth_mock.assert();
        
        // Now make an API call, which should detect the expired token and refresh
        let result = client.get_buyer_stats().await;
        assert!(result.is_ok());
        
        second_auth_mock.assert();
        api_mock.assert();
    }

    // Test health check
    #[tokio::test]
    async fn test_health_check() {
        let mock_server = mock("GET", "/health")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "healthy"}"#)
            .create();

        let mut config = ApiClientConfig::default();
        config.api_url = server_url();

        let client = LumarisApiClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        let result = client.health_check().await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        mock_server.assert();
    }
}

