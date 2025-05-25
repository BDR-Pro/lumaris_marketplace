use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use log::{info, error, debug};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use thiserror::Error;

#[cfg(test)]
mod tests;

// VM Manager errors
#[derive(Error, Debug)]
pub enum VmError {
    #[error("Failed to create VM: {0}")]
    CreationError(String),
    
    #[error("Failed to start VM: {0}")]
    StartError(String),
    
    #[error("Failed to stop VM: {0}")]
    StopError(String),
    
    #[error("VM not found: {0}")]
    NotFoundError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
}

// VM status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VmStatus {
    Creating,
    Running,
    Stopped,
    Failed,
    Terminated,
}

// VM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub vm_id: String,
    pub job_id: u64,
    pub buyer_id: String,
    pub vcpu_count: u32,
    pub mem_size_mib: u32,
    pub kernel_image_path: String,
    pub rootfs_path: String,
    pub socket_path: String,
}

// VM instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInstance {
    pub config: VmConfig,
    pub status: VmStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub ip_address: Option<String>,
    pub error_message: Option<String>,
}

// VM Manager
#[derive(Clone)]
pub struct VmManager {
    vms: Arc<Mutex<Vec<VmInstance>>>,
    pub vm_base_path: PathBuf,
    api_url: String,
}

impl VmManager {
    // Create a new VM Manager
    pub fn new(vm_base_path: &str, api_url: &str) -> Self {
        let vm_base_path = PathBuf::from(vm_base_path);
        
        // Create VM directory if it doesn't exist
        if !vm_base_path.exists() {
            fs::create_dir_all(&vm_base_path).unwrap_or_else(|e| {
                error!("Failed to create VM directory: {}", e);
            });
        }
        
        Self {
            vms: Arc::new(Mutex::new(Vec::new())),
            vm_base_path,
            api_url: api_url.to_string(),
        }
    }
    
    // Create a new VM for a job
    pub async fn create_vm(&self, job_id: u64, buyer_id: &str, vcpu_count: u32, mem_size_mib: u32) -> Result<String, VmError> {
        // Generate a unique VM ID
        let vm_id = Uuid::new_v4().to_string();
        
        // Create VM directory
        let vm_dir = self.vm_base_path.join(&vm_id);
        fs::create_dir_all(&vm_dir)?;
        
        // Define paths for VM files
        let kernel_path = vm_dir.join("vmlinux");
        let rootfs_path = vm_dir.join("rootfs.ext4");
        let socket_path = vm_dir.join("firecracker.sock");
        
        // Download kernel and rootfs (in a real implementation, these would be cached)
        self.download_vm_images(&kernel_path, &rootfs_path).await?;
        
        // Create VM config
        let config = VmConfig {
            vm_id: vm_id.clone(),
            job_id,
            buyer_id: buyer_id.to_string(),
            vcpu_count,
            mem_size_mib,
            kernel_image_path: kernel_path.to_string_lossy().to_string(),
            rootfs_path: rootfs_path.to_string_lossy().to_string(),
            socket_path: socket_path.to_string_lossy().to_string(),
        };
        
        // Create VM instance
        let now = chrono::Utc::now().timestamp() as u64;
        let vm = VmInstance {
            config: config.clone(),
            status: VmStatus::Creating,
            created_at: now,
            updated_at: now,
            ip_address: None,
            error_message: None,
        };
        
        // Add VM to list
        {
            let mut vms = self.vms.lock().await;
            vms.push(vm.clone());
        }
        
        // Start VM in background
        let vm_manager = self.clone();
        tokio::spawn(async move {
            if let Err(e) = vm_manager.start_vm(&config).await {
                error!("Failed to start VM {}: {}", vm_id, e);
                vm_manager.update_vm_status(&vm_id, VmStatus::Failed, Some(e.to_string())).await;
            } else {
                vm_manager.update_vm_status(&vm_id, VmStatus::Running, None).await;
            }
        });
        
        Ok(vm_id)
    }
    
    // Download VM images (kernel and rootfs)
    async fn download_vm_images(&self, kernel_path: &Path, rootfs_path: &Path) -> Result<(), VmError> {
        // In a real implementation, these would be downloaded from a repository
        // For now, we'll just create dummy files
        
        // Check if kernel already exists
        if !kernel_path.exists() {
            debug!("Downloading kernel image to {}", kernel_path.display());
            
            // In a real implementation, download the kernel
            // For now, just create a dummy file
            let mut file = File::create(kernel_path)?;
            file.write_all(b"dummy kernel")?;
        }
        
        // Check if rootfs already exists
        if !rootfs_path.exists() {
            debug!("Downloading rootfs image to {}", rootfs_path.display());
            
            // In a real implementation, download the rootfs
            // For now, just create a dummy file
            let mut file = File::create(rootfs_path)?;
            file.write_all(b"dummy rootfs")?;
        }
        
        Ok(())
    }
    
    // Start a VM using Firecracker
    async fn start_vm(&self, config: &VmConfig) -> Result<(), VmError> {
        // Create Firecracker configuration
        let fc_config = json!({
            "boot-source": {
                "kernel_image_path": config.kernel_image_path,
                "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
            },
            "drives": [
                {
                    "drive_id": "rootfs",
                    "path_on_host": config.rootfs_path,
                    "is_root_device": true,
                    "is_read_only": false
                }
            ],
            "machine-config": {
                "vcpu_count": config.vcpu_count,
                "mem_size_mib": config.mem_size_mib
            },
            "network-interfaces": [
                {
                    "iface_id": "eth0",
                    "guest_mac": "AA:FC:00:00:00:01",
                    "host_dev_name": "tap0"
                }
            ]
        });
        
        // Write configuration to file
        let config_path = Path::new(&config.socket_path).with_extension("json");
        let mut file = File::create(&config_path)?;
        file.write_all(fc_config.to_string().as_bytes())?;
        
        // Remove socket file if it exists
        if Path::new(&config.socket_path).exists() {
            fs::remove_file(&config.socket_path)?;
        }
        
        // Start Firecracker process
        info!("Starting Firecracker VM {}", config.vm_id);
        
        // In a real implementation, we would use the Firecracker API to start the VM
        // For now, we'll just simulate it with a command
        let output = Command::new("echo")
            .arg(format!("Starting Firecracker VM with config: {}", config_path.display()))
            .output()?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
            error!("Failed to start Firecracker VM: {}", error_msg);
            return Err(VmError::StartError(error_msg));
        }
        
        // In a real implementation, we would wait for the VM to start and get its IP address
        // For now, we'll just simulate it
        let ip_address = format!("192.168.100.{}", rand::random::<u8>());
        
        // Update VM with IP address
        self.update_vm_ip(&config.vm_id, &ip_address).await;
        
        info!("Firecracker VM {} started with IP {}", config.vm_id, ip_address);
        
        Ok(())
    }
    
    // Stop a VM
    pub async fn stop_vm(&self, vm_id: &str) -> Result<(), VmError> {
        // Find VM
        let vm = self.get_vm(vm_id).await?;
        
        // Check if VM is already stopped
        if vm.status == VmStatus::Stopped || vm.status == VmStatus::Terminated {
            return Ok(());
        }
        
        info!("Stopping Firecracker VM {}", vm_id);
        
        // In a real implementation, we would use the Firecracker API to stop the VM
        // For now, we'll just simulate it
        let output = Command::new("echo")
            .arg(format!("Stopping Firecracker VM {}", vm_id))
            .output()?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
            error!("Failed to stop Firecracker VM: {}", error_msg);
            return Err(VmError::StopError(error_msg));
        }
        
        // Update VM status
        self.update_vm_status(vm_id, VmStatus::Stopped, None).await;
        
        info!("Firecracker VM {} stopped", vm_id);
        
        Ok(())
    }
    
    // Terminate a VM (stop and clean up)
    pub async fn terminate_vm(&self, vm_id: &str) -> Result<(), VmError> {
        // Stop VM first
        let _ = self.stop_vm(vm_id).await;
        
        // Find VM
        let vm = self.get_vm(vm_id).await?;
        
        info!("Terminating Firecracker VM {}", vm_id);
        
        // Clean up VM files
        let vm_dir = self.vm_base_path.join(vm_id);
        if vm_dir.exists() {
            fs::remove_dir_all(&vm_dir)?;
        }
        
        // Update VM status
        self.update_vm_status(vm_id, VmStatus::Terminated, None).await;
        
        info!("Firecracker VM {} terminated", vm_id);
        
        Ok(())
    }
    
    // Get VM by ID
    pub async fn get_vm(&self, vm_id: &str) -> Result<VmInstance, VmError> {
        let vms = self.vms.lock().await;
        
        for vm in vms.iter() {
            if vm.config.vm_id == vm_id {
                return Ok(vm.clone());
            }
        }
        
        Err(VmError::NotFoundError(vm_id.to_string()))
    }
    
    // Get all VMs
    pub async fn get_all_vms(&self) -> Vec<VmInstance> {
        let vms = self.vms.lock().await;
        vms.clone()
    }
    
    // Get VMs by buyer ID
    pub async fn get_vms_by_buyer(&self, buyer_id: &str) -> Vec<VmInstance> {
        let vms = self.vms.lock().await;
        
        vms.iter()
            .filter(|vm| vm.config.buyer_id == buyer_id)
            .cloned()
            .collect()
    }
    
    // Get VMs by job ID
    pub async fn get_vms_by_job(&self, job_id: u64) -> Vec<VmInstance> {
        let vms = self.vms.lock().await;
        
        vms.iter()
            .filter(|vm| vm.config.job_id == job_id)
            .cloned()
            .collect()
    }
    
    // Update VM status
    async fn update_vm_status(&self, vm_id: &str, status: VmStatus, error_message: Option<String>) {
        let mut vms = self.vms.lock().await;
        
        for vm in vms.iter_mut() {
            if vm.config.vm_id == vm_id {
                vm.status = status.clone();
                vm.updated_at = chrono::Utc::now().timestamp() as u64;
                vm.error_message = error_message.clone();
                
                // Notify API about VM status change
                let vm_clone = vm.clone();
                let api_url = self.api_url.clone();
                tokio::spawn(async move {
                    if let Err(e) = update_vm_status_in_api(&api_url, &vm_clone).await {
                        error!("Failed to update VM status in API: {}", e);
                    }
                });
                
                break;
            }
        }
    }
    
    // Update VM IP address
    async fn update_vm_ip(&self, vm_id: &str, ip_address: &str) {
        let mut vms = self.vms.lock().await;
        
        for vm in vms.iter_mut() {
            if vm.config.vm_id == vm_id {
                vm.ip_address = Some(ip_address.to_string());
                vm.updated_at = chrono::Utc::now().timestamp() as u64;
                
                // Notify API about VM IP change
                let vm_clone = vm.clone();
                let api_url = self.api_url.clone();
                tokio::spawn(async move {
                    if let Err(e) = update_vm_status_in_api(&api_url, &vm_clone).await {
                        error!("Failed to update VM IP in API: {}", e);
                    }
                });
                
                break;
            }
        }
    }
}

// Update VM status in API
async fn update_vm_status_in_api(api_url: &str, vm: &VmInstance) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    
    let payload = json!({
        "vm_id": vm.config.vm_id,
        "job_id": vm.config.job_id,
        "buyer_id": vm.config.buyer_id,
        "status": format!("{:?}", vm.status),
        "ip_address": vm.ip_address,
        "error_message": vm.error_message
    });
    
    client.post(&format!("{}/api/vms/{}/status", api_url, vm.config.vm_id))
        .json(&payload)
        .send()
        .await?;
    
    Ok(())
}

