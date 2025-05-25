use super::*;
use std::env;
use std::path::PathBuf;
use tokio::runtime::Runtime;
use tempfile::tempdir;

// Helper function to create a test VM manager
fn create_test_vm_manager() -> VmManager {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let vm_base_path = temp_dir.path().to_str().unwrap();
    VmManager::new(vm_base_path, "http://localhost:8000")
}

#[test]
fn test_vm_manager_creation() {
    let vm_manager = create_test_vm_manager();
    assert!(vm_manager.vm_base_path.exists());
}

#[test]
fn test_vm_creation() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let vm_manager = create_test_vm_manager();
        
        // Create a VM
        let result = vm_manager.create_vm(123, "test-buyer", 2, 1024).await;
        assert!(result.is_ok());
        
        let vm_id = result.unwrap();
        assert!(!vm_id.is_empty());
        
        // Check if VM exists
        let vm = vm_manager.get_vm(&vm_id).await;
        assert!(vm.is_ok());
        
        let vm = vm.unwrap();
        assert_eq!(vm.config.job_id, 123);
        assert_eq!(vm.config.buyer_id, "test-buyer");
        assert_eq!(vm.config.vcpu_count, 2);
        assert_eq!(vm.config.mem_size_mib, 1024);
    });
}

#[test]
fn test_vm_lifecycle() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let vm_manager = create_test_vm_manager();
        
        // Create a VM
        let vm_id = vm_manager.create_vm(123, "test-buyer", 2, 1024).await.unwrap();
        
        // Wait for VM to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Get VM status
        let vm = vm_manager.get_vm(&vm_id).await.unwrap();
        assert!(vm.status == VmStatus::Creating || vm.status == VmStatus::Running);
        
        // Stop VM
        let result = vm_manager.stop_vm(&vm_id).await;
        assert!(result.is_ok());
        
        // Check VM status
        let vm = vm_manager.get_vm(&vm_id).await.unwrap();
        assert_eq!(vm.status, VmStatus::Stopped);
        
        // Terminate VM
        let result = vm_manager.terminate_vm(&vm_id).await;
        assert!(result.is_ok());
        
        // Check VM status
        let vm = vm_manager.get_vm(&vm_id).await.unwrap();
        assert_eq!(vm.status, VmStatus::Terminated);
    });
}

#[test]
fn test_get_vms_by_buyer() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let vm_manager = create_test_vm_manager();
        
        // Create VMs for different buyers
        let vm_id1 = vm_manager.create_vm(123, "buyer-1", 2, 1024).await.unwrap();
        let vm_id2 = vm_manager.create_vm(124, "buyer-1", 4, 2048).await.unwrap();
        let vm_id3 = vm_manager.create_vm(125, "buyer-2", 2, 1024).await.unwrap();
        
        // Get VMs for buyer-1
        let vms = vm_manager.get_vms_by_buyer("buyer-1").await;
        assert_eq!(vms.len(), 2);
        
        // Get VMs for buyer-2
        let vms = vm_manager.get_vms_by_buyer("buyer-2").await;
        assert_eq!(vms.len(), 1);
        
        // Get VMs for non-existent buyer
        let vms = vm_manager.get_vms_by_buyer("buyer-3").await;
        assert_eq!(vms.len(), 0);
    });
}

#[test]
fn test_get_vms_by_job() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let vm_manager = create_test_vm_manager();
        
        // Create VMs for different jobs
        let vm_id1 = vm_manager.create_vm(123, "buyer-1", 2, 1024).await.unwrap();
        let vm_id2 = vm_manager.create_vm(124, "buyer-1", 4, 2048).await.unwrap();
        let vm_id3 = vm_manager.create_vm(123, "buyer-2", 2, 1024).await.unwrap();
        
        // Get VMs for job 123
        let vms = vm_manager.get_vms_by_job(123).await;
        assert_eq!(vms.len(), 2);
        
        // Get VMs for job 124
        let vms = vm_manager.get_vms_by_job(124).await;
        assert_eq!(vms.len(), 1);
        
        // Get VMs for non-existent job
        let vms = vm_manager.get_vms_by_job(999).await;
        assert_eq!(vms.len(), 0);
    });
}

#[test]
fn test_vm_error_handling() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let vm_manager = create_test_vm_manager();
        
        // Try to get a non-existent VM
        let result = vm_manager.get_vm("non-existent-vm").await;
        assert!(result.is_err());
        
        // Try to stop a non-existent VM
        let result = vm_manager.stop_vm("non-existent-vm").await;
        assert!(result.is_err());
        
        // Try to terminate a non-existent VM
        let result = vm_manager.terminate_vm("non-existent-vm").await;
        assert!(result.is_err());
    });
}

