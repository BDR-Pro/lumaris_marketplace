#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tokio::time::sleep;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;
    use futures_util::StreamExt;
    use futures_util::SinkExt;
    use serde_json::json;

    // Test WebSocket message creation
    #[test]
    fn test_ws_message_creation() {
        let role = UserRole::Buyer;
        let data = json!({"test": "data"});
        let message = WsMessage::new("test_type", &role, data.clone());
        
        assert_eq!(message.type_, "test_type");
        assert_eq!(message.role, "buyer");
        assert_eq!(message.data, data);
        assert!(!message.request_id.is_empty());
    }

    // Test WebSocket message serialization
    #[test]
    fn test_ws_message_serialization() {
        let role = UserRole::Seller;
        let data = json!({"key": "value"});
        let message = WsMessage::new("test_type", &role, data);
        
        let json_str = message.to_json().unwrap();
        
        // Verify the JSON contains the correct fields
        assert!(json_str.contains("\"type\":\"test_type\""));
        assert!(json_str.contains("\"role\":\"seller\""));
        assert!(json_str.contains("\"key\":\"value\""));
    }

    // Test WebSocket message deserialization
    #[test]
    fn test_ws_message_deserialization() {
        let json_str = r#"{
            "type": "test_type",
            "request_id": "test-id",
            "role": "buyer",
            "data": {"key": "value"},
            "timestamp": "2023-01-01T00:00:00Z"
        }"#;
        
        let message = WsMessage::from_json(json_str).unwrap();
        
        assert_eq!(message.type_, "test_type");
        assert_eq!(message.request_id, "test-id");
        assert_eq!(message.role, "buyer");
        assert_eq!(message.data["key"], "value");
    }

    // Test WebSocket client configuration
    #[test]
    fn test_ws_client_config_default() {
        let config = WsClientConfig::default();
        assert_eq!(config.ws_url, "ws://127.0.0.1:3030/ws");
        assert_eq!(config.reconnect_delay_ms, 1000);
        assert_eq!(config.max_reconnect_attempts, 10);
        assert_eq!(config.heartbeat_interval_ms, 30000);
        assert_eq!(config.message_timeout_ms, 5000);
    }

    // Test WebSocket client creation
    #[test]
    fn test_ws_client_creation() {
        let config = WsClientConfig::default();
        let client = LumarisWsClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        
        assert_eq!(client.role, UserRole::Buyer);
        assert_eq!(client.user_id, "test-buyer");
    }

    // Test WebSocket error creation
    #[test]
    fn test_ws_error_creation() {
        let error = WsError::ConnectionError("Failed to connect".to_string());
        assert!(error.to_string().contains("Failed to connect"));
        
        let error = WsError::Timeout;
        assert_eq!(error.to_string(), "Timeout");
    }

    // Test WebSocket client connection and message handling
    #[tokio::test]
    async fn test_ws_client_connection() {
        // Start a WebSocket server
        let server = start_test_ws_server().await;
        let server_addr = server.local_addr().unwrap();
        let server_url = format!("ws://127.0.0.1:{}/ws", server_addr.port());
        
        // Create a WebSocket client
        let mut config = WsClientConfig::default();
        config.ws_url = server_url;
        config.heartbeat_interval_ms = 100; // Fast heartbeat for testing
        
        let client = LumarisWsClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        
        // Start the client
        client.start().await.unwrap();
        
        // Wait for connection
        client.wait_for_connection(1000).await.unwrap();
        
        // Verify connection
        assert!(client.is_connected().await);
        
        // Send a message
        client.send_message("test_message", json!({"test": "data"})).unwrap();
        
        // Wait for message processing
        sleep(Duration::from_millis(200)).await;
        
        // Clean up
        drop(server);
    }

    // Test WebSocket client message handlers
    #[tokio::test]
    async fn test_ws_client_message_handlers() {
        // Create a message received flag
        let message_received = Arc::new(Mutex::new(false));
        
        // Start a WebSocket server that sends a message
        let server = start_test_ws_server_with_message().await;
        let server_addr = server.local_addr().unwrap();
        let server_url = format!("ws://127.0.0.1:{}/ws", server_addr.port());
        
        // Create a WebSocket client
        let mut config = WsClientConfig::default();
        config.ws_url = server_url;
        
        let client = LumarisWsClient::new(config, UserRole::Buyer, "test-buyer".to_string());
        
        // Register a message handler
        let message_received_clone = message_received.clone();
        client.register_handler(move |message| {
            if message.type_ == "test_message" {
                let mut flag = message_received_clone.blocking_lock();
                *flag = true;
                return true;
            }
            false
        }).await;
        
        // Start the client
        client.start().await.unwrap();
        
        // Wait for connection and message
        sleep(Duration::from_millis(500)).await;
        
        // Verify message was received
        let flag = message_received.lock().await;
        assert!(*flag);
        
        // Clean up
        drop(server);
    }

    // Test WebSocket client role validation
    #[test]
    fn test_ws_client_role_validation() {
        let config = WsClientConfig::default();
        
        // Test buyer operations
        let buyer_client = LumarisWsClient::new(config.clone(), UserRole::Buyer, "test-buyer".to_string());
        assert!(buyer_client.create_vm(123, 2, 1024).is_ok());
        assert!(buyer_client.register_node(4.0, 8192).is_err());
        
        // Test seller operations
        let seller_client = LumarisWsClient::new(config, UserRole::Seller, "test-seller".to_string());
        assert!(seller_client.create_vm(123, 2, 1024).is_err());
        assert!(seller_client.register_node(4.0, 8192).is_ok());
    }

    // Helper function to start a test WebSocket server
    async fn start_test_ws_server() -> TcpListener {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        
        let listener_clone = listener.try_clone().unwrap();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener_clone.accept().await {
                tokio::spawn(async move {
                    let ws_stream = accept_async(stream).await.unwrap();
                    let (mut write, mut read) = ws_stream.split();
                    
                    // Echo messages back
                    while let Some(Ok(msg)) = read.next().await {
                        if msg.is_text() || msg.is_binary() {
                            write.send(msg).await.unwrap();
                        }
                    }
                });
            }
        });
        
        listener
    }

    // Helper function to start a test WebSocket server that sends a message
    async fn start_test_ws_server_with_message() -> TcpListener {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        
        let listener_clone = listener.try_clone().unwrap();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener_clone.accept().await {
                tokio::spawn(async move {
                    let ws_stream = accept_async(stream).await.unwrap();
                    let (mut write, _) = ws_stream.split();
                    
                    // Send a test message
                    let test_message = json!({
                        "type": "test_message",
                        "request_id": "test-id",
                        "role": "server",
                        "data": {"key": "value"},
                        "timestamp": "2023-01-01T00:00:00Z"
                    });
                    
                    write.send(Message::Text(test_message.to_string())).await.unwrap();
                });
            }
        });
        
        listener
    }
}

