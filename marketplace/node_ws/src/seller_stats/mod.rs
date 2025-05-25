use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use log::{info, debug};
use colored::*;
use rand::Rng;

// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub vm_count: u32,
    pub earnings: f64,
    pub uptime_hours: f64,
    pub reliability_score: f32,
}

// VM hosting record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmHostingRecord {
    pub vm_id: String,
    pub job_id: u64,
    pub buyer_id: String,
    pub node_id: String,
    pub vcpu_count: u32,
    pub mem_size_mib: u32,
    pub created_at: DateTime<Utc>,
    pub terminated_at: Option<DateTime<Utc>>,
    pub earnings: f64,
    pub status: String,
}

// Seller session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellerSessionStats {
    pub seller_id: String,
    pub session_start: DateTime<Utc>,
    pub active_nodes: u32,
    pub total_nodes: u32,
    pub active_vms: u32,
    pub total_vms_hosted: u32,
    pub total_earnings: f64,
    pub total_cpu_cores: f32,
    pub total_memory_mb: u64,
    pub nodes: HashMap<String, NodeInfo>,
    pub hosting_records: Vec<VmHostingRecord>,
    pub buyer_distribution: HashMap<String, u32>,
}

// Seller statistics manager
#[derive(Clone)]
pub struct SellerStatsManager {
    stats: Arc<Mutex<HashMap<String, SellerSessionStats>>>,
}

impl SellerStatsManager {
    // Create a new seller statistics manager
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    // Get or create seller session stats
    async fn get_or_create_seller_stats(&self, seller_id: &str) -> SellerSessionStats {
        let mut stats = self.stats.lock().await;
        
        if !stats.contains_key(seller_id) {
            let new_stats = SellerSessionStats {
                seller_id: seller_id.to_string(),
                session_start: Utc::now(),
                active_nodes: 0,
                total_nodes: 0,
                active_vms: 0,
                total_vms_hosted: 0,
                total_earnings: 0.0,
                total_cpu_cores: 0.0,
                total_memory_mb: 0,
                nodes: HashMap::new(),
                hosting_records: Vec::new(),
                buyer_distribution: HashMap::new(),
            };
            
            stats.insert(seller_id.to_string(), new_stats.clone());
            return new_stats;
        }
        
        stats.get(seller_id).unwrap().clone()
    }
    
    // Record node registration
    pub async fn record_node_registration(&self, seller_id: &str, node_id: &str, cpu_cores: f32, memory_mb: u64) {
        let mut stats = self.stats.lock().await;
        
        // Get or create seller stats
        if !stats.contains_key(seller_id) {
            let new_stats = SellerSessionStats {
                seller_id: seller_id.to_string(),
                session_start: Utc::now(),
                active_nodes: 0,
                total_nodes: 0,
                active_vms: 0,
                total_vms_hosted: 0,
                total_earnings: 0.0,
                total_cpu_cores: 0.0,
                total_memory_mb: 0,
                nodes: HashMap::new(),
                hosting_records: Vec::new(),
                buyer_distribution: HashMap::new(),
            };
            
            stats.insert(seller_id.to_string(), new_stats);
        }
        
        // Get seller stats
        let seller_stats = stats.get_mut(seller_id).unwrap();
        
        // Create node info
        let node_info = NodeInfo {
            node_id: node_id.to_string(),
            cpu_cores,
            memory_mb,
            vm_count: 0,
            earnings: 0.0,
            uptime_hours: 0.0,
            reliability_score: 1.0,
        };
        
        // Update seller stats
        if !seller_stats.nodes.contains_key(node_id) {
            seller_stats.active_nodes += 1;
            seller_stats.total_nodes += 1;
            seller_stats.total_cpu_cores += cpu_cores;
            seller_stats.total_memory_mb += memory_mb;
        }
        
        // Update node info
        seller_stats.nodes.insert(node_id.to_string(), node_info);
        
        info!("{} {} {} {} {} {} {} {}", 
            "Node".green(), 
            node_id.bright_yellow(), 
            "registered for seller".green(), 
            seller_id.bright_cyan(), 
            "with".green(),
            cpu_cores.to_string().bright_white(),
            "CPU cores and".green(),
            format!("{} MB", memory_mb).bright_white()
        );
    }
    
    // Record VM hosting
    pub async fn record_vm_hosting(&self, seller_id: &str, node_id: &str, vm_id: &str, job_id: u64, buyer_id: &str, vcpu_count: u32, mem_size_mib: u32) {
        let mut stats = self.stats.lock().await;
        
        // Check if seller exists
        if !stats.contains_key(seller_id) {
            debug!("Seller {} not found in stats", seller_id);
            return;
        }
        
        // Get seller stats
        let seller_stats = stats.get_mut(seller_id).unwrap();
        
        // Check if node exists
        if !seller_stats.nodes.contains_key(node_id) {
            debug!("Node {} not found in seller stats", node_id);
            return;
        }
        
        // Create VM hosting record
        let hosting_record = VmHostingRecord {
            vm_id: vm_id.to_string(),
            job_id,
            buyer_id: buyer_id.to_string(),
            node_id: node_id.to_string(),
            vcpu_count,
            mem_size_mib,
            created_at: Utc::now(),
            terminated_at: None,
            earnings: 0.0,
            status: "running".to_string(),
        };
        
        // Update seller stats
        seller_stats.active_vms += 1;
        seller_stats.total_vms_hosted += 1;
        seller_stats.hosting_records.push(hosting_record);
        
        // Update node stats
        if let Some(node) = seller_stats.nodes.get_mut(node_id) {
            node.vm_count += 1;
        }
        
        // Update buyer distribution
        *seller_stats.buyer_distribution.entry(buyer_id.to_string()).or_insert(0) += 1;
        
        info!("{} {} {} {} {} {} {}", 
            "VM".green(), 
            vm_id.bright_yellow(), 
            "hosted on node".green(), 
            node_id.bright_yellow(), 
            "for buyer".green(),
            buyer_id.bright_cyan(),
            "by seller".green()
        );
    }
    
    // Record VM termination
    pub async fn record_vm_termination(&self, seller_id: &str, vm_id: &str) {
        let mut stats = self.stats.lock().await;
        
        // Check if seller exists
        if !stats.contains_key(seller_id) {
            debug!("Seller {} not found in stats", seller_id);
            return;
        }
        
        // Get seller stats
        let seller_stats = stats.get_mut(seller_id).unwrap();
        
        // Find VM hosting record
        for record in &mut seller_stats.hosting_records {
            if record.vm_id == vm_id {
                // Update record
                record.terminated_at = Some(Utc::now());
                record.status = "terminated".to_string();
                
                // Calculate runtime in hours
                let runtime_hours = (Utc::now() - record.created_at).num_seconds() as f64 / 3600.0;
                
                // Calculate earnings
                let hourly_rate = (record.vcpu_count as f64 * 0.05) + (record.mem_size_mib as f64 * 0.0001);
                let earnings = hourly_rate * runtime_hours;
                record.earnings = earnings;
                
                // Update seller stats
                seller_stats.active_vms -= 1;
                seller_stats.total_earnings += earnings;
                
                // Update node stats
                if let Some(node) = seller_stats.nodes.get_mut(&record.node_id) {
                    node.earnings += earnings;
                }
                
                info!("{} {} {} {} {} ${:.2}", 
                    "VM".yellow(), 
                    vm_id.bright_yellow(), 
                    "terminated on node".yellow(), 
                    record.node_id.bright_yellow(), 
                    "earnings:".green(),
                    earnings
                );
                
                break;
            }
        }
    }
    
    // Record node disconnection
    pub async fn record_node_disconnection(&self, seller_id: &str, node_id: &str) {
        let mut stats = self.stats.lock().await;
        
        // Check if seller exists
        if !stats.contains_key(seller_id) {
            debug!("Seller {} not found in stats", seller_id);
            return;
        }
        
        // Get seller stats
        let seller_stats = stats.get_mut(seller_id).unwrap();
        
        // Check if node exists
        if !seller_stats.nodes.contains_key(node_id) {
            debug!("Node {} not found in seller stats", node_id);
            return;
        }
        
        // Update seller stats
        seller_stats.active_nodes -= 1;
        
        // Calculate node uptime
        if let Some(node) = seller_stats.nodes.get_mut(node_id) {
            // Calculate uptime in hours (simulated for now)
            let uptime_hours = rand::thread_rng().gen_range(1.0..24.0);
            node.uptime_hours += uptime_hours;
            
            info!("{} {} {} {} {} {}", 
                "Node".yellow(), 
                node_id.bright_yellow(), 
                "disconnected for seller".yellow(), 
                seller_id.bright_cyan(), 
                "uptime:".green(),
                format!("{:.2} hours", uptime_hours).bright_white()
            );
        }
    }
    
    // Get seller statistics
    pub async fn get_seller_stats(&self, seller_id: &str) -> Option<SellerSessionStats> {
        let stats = self.stats.lock().await;
        stats.get(seller_id).cloned()
    }
    
    // Get all seller statistics
    pub async fn get_all_seller_stats(&self) -> HashMap<String, SellerSessionStats> {
        let stats = self.stats.lock().await;
        stats.clone()
    }
}

// Format seller statistics as a pretty string
pub fn format_seller_stats(stats: &SellerSessionStats) -> String {
    let mut result = String::new();
    
    // Format session info
    result.push_str(&format!("🧑‍💻 Seller: {}\n", stats.seller_id.bright_cyan()));
    result.push_str(&format!("⏱️ Session started: {}\n", stats.session_start.format("%Y-%m-%d %H:%M:%S UTC")));
    result.push_str(&format!("🖥️ Active Nodes: {}/{}\n", stats.active_nodes.to_string().bright_yellow(), stats.total_nodes.to_string().bright_yellow()));
    result.push_str(&format!("🔄 Active VMs: {}/{}\n", stats.active_vms.to_string().bright_yellow(), stats.total_vms_hosted.to_string().bright_yellow()));
    result.push_str(&format!("💰 Total earnings: ${:.2}\n", stats.total_earnings));
    result.push_str(&format!("💻 Total resources: {} CPU cores, {} MB RAM\n\n", 
        stats.total_cpu_cores.to_string().bright_yellow(),
        stats.total_memory_mb.to_string().bright_yellow()
    ));
    
    // Format node info
    result.push_str("🖥️ Nodes:\n");
    
    // Sort nodes by VM count
    let mut nodes: Vec<&NodeInfo> = stats.nodes.values().collect();
    nodes.sort_by(|a, b| b.vm_count.cmp(&a.vm_count));
    
    for node in nodes {
        result.push_str(&format!("  📡 Node {} - {} VMs, {} CPU cores, {} MB RAM, ${:.2} earnings\n", 
            node.node_id.bright_yellow(),
            node.vm_count.to_string().bright_white(),
            node.cpu_cores.to_string().bright_white(),
            node.memory_mb.to_string().bright_white(),
            node.earnings
        ));
    }
    
    // Format buyer distribution
    result.push_str("\n👥 Buyer Distribution:\n");
    
    // Sort buyers by VM count
    let mut buyers: Vec<(&String, &u32)> = stats.buyer_distribution.iter().collect();
    buyers.sort_by(|a, b| b.1.cmp(a.1));
    
    for (buyer_id, count) in buyers {
        result.push_str(&format!("  👤 Buyer {} - {} VMs\n", 
            buyer_id.bright_cyan(),
            count.to_string().bright_yellow()
        ));
    }
    
    // Format recent hosting records
    result.push_str("\n🔄 Recent Hosting Records:\n");
    
    // Get the 5 most recent records
    let mut records = stats.hosting_records.clone();
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let recent_records = records.iter().take(5);
    
    for record in recent_records {
        let status_str = match record.status.as_str() {
            "running" => "Running".bright_green(),
            "terminated" => "Terminated".bright_red(),
            _ => record.status.normal(),
        };
        
        result.push_str(&format!("  VM {} - Job {} - Buyer {} - {} - ${:.2}\n", 
            record.vm_id.bright_yellow(),
            record.job_id,
            record.buyer_id.bright_cyan(),
            status_str,
            record.earnings
        ));
    }
    
    result
}

