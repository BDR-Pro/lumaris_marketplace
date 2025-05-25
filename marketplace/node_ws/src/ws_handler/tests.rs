use super::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use warp::ws::Message;
use serde_json::json;
use crate::matchmaker::create_matchmaker;
use crate::vm_manager::VmManager;
use tempfile::tempdir;

// Helper function to create a test WebSocket message
fn create_ws_message(message_type: &str, data: serde_json::Value) -> Message {
    let mut message = json!({
        "type": message_type
    });
    
    // Merge the data into the message
    if let Some(obj) = message.as_object_mut() {
        if let Some(data_obj) = data.as_object() {
            for (key, value) in data_obj {
                obj.insert(key.clone(), value.clone());
            }
        }
    }
    
    Message::text(message.to_string())
}

// Test processing a node registration message
#[test]
fn test_process_node_registration() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Create test components
        let matchmaker = create_matchmaker();
        let connections: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>> = Arc::new(Mutex::new(HashMap::new()));
        let (tx, _rx) = mpsc::unbounded_channel::<Message>();
        
        // Create a temporary directory for VM files
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let vm_base_path = temp_dir.path().to_str().unwrap();
        let vm_manager = VmManager::new(vm_base_path, "http://localhost:8000");
        
        // Create a node registration message
        let message = json!({
            "type": "node_registration",
            "capabilities": {
                "cpu_cores": 4,
                "memory_mb": 8192
            }
        }).to_string();
        
        // Process the message
        let result = process_message(
            &message,
            "test-node-id",
            "http://localhost:8000",
            &matchmaker,
            &connections,
            &tx,
            &vm_manager
        ).await;
        
        // Check that the message was processed successfully
        assert!(result.is_ok());
        
        // Check that the node was registered with the matchmaker
        let mm = matchmaker.lock().await;
        let nodes = mm.get_all_nodes();
        
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, "test-node-id");
        assert_eq!(nodes[0].cpu_cores, 4.0);
        assert_eq!(nodes[0].memory_mb, 8192);
        assert!(nodes[0].available);
    });
}

// Test processing a VM creation message
#[test]
fn test_process_vm_creation() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Create test components
        let matchmaker = create_matchmaker();
        let connections: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>> = Arc::new(Mutex::new(HashMap::new()));
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        
        // Create a temporary directory for VM files
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let vm_base_path = temp_dir.path().to_str().unwrap();
        let vm_manager = VmManager::new(vm_base_path, "http://localhost:8000");
        
        // Create a VM creation message
        let message = json!({
            "type": "create_vm",
            "job_id": 123,
            "buyer_id": "test-buyer",
            "vcpu_count": 2,
            "mem_size_mib": 1024
        }).to_string();
        
        // Process the message
        let result = process_message(
            &message,
            "test-node-id",
            "http://localhost:8000",
            &matchmaker,
            &connections,
            &tx,
            &vm_manager
        ).await;
        
        // Check that the message was processed successfully
        assert!(result.is_ok());
        
        // Check that a VM was created
        let vms = vm_manager.get_vms_by_buyer("test-buyer").await;
        assert_eq!(vms.len(), 1);
        assert_eq!(vms[0].config.job_id, 123);
        assert_eq!(vms[0].config.buyer_id, "test-buyer");
        assert_eq!(vms[0].config.vcpu_count, 2);
        assert_eq!(vms[0].config.mem_size_mib, 1024);
        
        // Check that a response was sent
        let response = rx.try_recv().unwrap();
        let response_text = response.to_str().unwrap();
        let response_json: serde_json::Value = serde_json::from_str(response_text).unwrap();
        
        assert_eq!(response_json["type"], "vm_created");
        assert_eq!(response_json["job_id"], 123);
        assert_eq!(response_json["buyer_id"], "test-buyer");
        assert!(response_json["vm_id"].is_string());
        assert_eq!(response_json["status"], "creating");
    });
}

// Test processing a VM status request
#[test]
fn test_process_vm_status_request() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Create test components
        let matchmaker = create_matchmaker();
        let connections: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>> = Arc::new(Mutex::new(HashMap::new()));
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        
        // Create a temporary directory for VM files
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let vm_base_path = temp_dir.path().to_str().unwrap();
        let vm_manager = VmManager::new(vm_base_path, "http://localhost:8000");
        
        // Create a VM
        let vm_id = vm_manager.create_vm(123, "test-buyer", 2, 1024).await.unwrap();
        
        // Create a VM status request message
        let message = json!({
            "type": "get_vm_status",
            "vm_id": vm_id
        }).to_string();
        
        // Process the message
        let result = process_message(
            &message,
            "test-node-id",
            "http://localhost:8000",
            &matchmaker,
            &connections,
            &tx,
            &vm_manager
        ).await;
        
        // Check that the message was processed successfully
        assert!(result.is_ok());
        
        // Check that a response was sent
        let response = rx.try_recv().unwrap();
        let response_text = response.to_str().unwrap();
        let response_json: serde_json::Value = serde_json::from_str(response_text).unwrap();
        
        assert_eq!(response_json["type"], "vm_status");
        assert_eq!(response_json["vm_id"], vm_id);
        assert_eq!(response_json["job_id"], 123);
        assert_eq!(response_json["buyer_id"], "test-buyer");
        assert!(response_json["status"].is_string());
    });
}

// Test processing a buyer VMs request
#[test]
fn test_process_buyer_vms_request() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Create test components
        let matchmaker = create_matchmaker();
        let connections: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>> = Arc::new(Mutex::new(HashMap::new()));
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        
        // Create a temporary directory for VM files
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let vm_base_path = temp_dir.path().to_str().unwrap();
        let vm_manager = VmManager::new(vm_base_path, "http://localhost:8000");
        
        // Create VMs for different buyers
        let vm_id1 = vm_manager.create_vm(123, "buyer-1", 2, 1024).await.unwrap();
        let vm_id2 = vm_manager.create_vm(124, "buyer-1", 4, 2048).await.unwrap();
        let vm_id3 = vm_manager.create_vm(125, "buyer-2", 2, 1024).await.unwrap();
        
        // Create a buyer VMs request message
        let message = json!({
            "type": "get_buyer_vms",
            "buyer_id": "buyer-1"
        }).to_string();
        
        // Process the message
        let result = process_message(
            &message,
            "test-node-id",
            "http://localhost:8000",
            &matchmaker,
            &connections,
            &tx,
            &vm_manager
        ).await;
        
        // Check that the message was processed successfully
        assert!(result.is_ok());
        
        // Check that a response was sent
        let response = rx.try_recv().unwrap();
        let response_text = response.to_str().unwrap();
        let response_json: serde_json::Value = serde_json::from_str(response_text).unwrap();
        
        assert_eq!(response_json["type"], "buyer_vms");
        assert_eq!(response_json["buyer_id"], "buyer-1");
        assert!(response_json["vms"].is_array());
        
        let vms = response_json["vms"].as_array().unwrap();
        assert_eq!(vms.len(), 2);
    });
}

