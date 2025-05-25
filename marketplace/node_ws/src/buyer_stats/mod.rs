use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use log::{info, debug};
use colored::*;
use rand::Rng;

// Country information with flag emoji
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryInfo {
    pub country_code: String,
    pub country_name: String,
    pub flag_emoji: String,
    pub vm_count: u32,
    pub earnings: f64,
}

// VM transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmTransaction {
    pub vm_id: String,
    pub job_id: u64,
    pub buyer_id: String,
    pub customer_country: CountryInfo,
    pub vcpu_count: u32,
    pub mem_size_mib: u32,
    pub created_at: DateTime<Utc>,
    pub terminated_at: Option<DateTime<Utc>>,
    pub earnings: f64,
    pub status: String,
}

// Buyer session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyerSessionStats {
    pub buyer_id: String,
    pub session_start: DateTime<Utc>,
    pub active_vms: u32,
    pub total_vms_spawned: u32,
    pub total_earnings: f64,
    pub countries: HashMap<String, CountryInfo>,
    pub transactions: Vec<VmTransaction>,
}

// Buyer statistics manager
#[derive(Clone)]
pub struct BuyerStatsManager {
    stats: Arc<Mutex<HashMap<String, BuyerSessionStats>>>,
}

impl BuyerStatsManager {
    // Create a new buyer statistics manager
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    // Get or create buyer session stats
    async fn get_or_create_buyer_stats(&self, buyer_id: &str) -> BuyerSessionStats {
        let mut stats = self.stats.lock().await;
        
        if !stats.contains_key(buyer_id) {
            let new_stats = BuyerSessionStats {
                buyer_id: buyer_id.to_string(),
                session_start: Utc::now(),
                active_vms: 0,
                total_vms_spawned: 0,
                total_earnings: 0.0,
                countries: HashMap::new(),
                transactions: Vec::new(),
            };
            
            stats.insert(buyer_id.to_string(), new_stats.clone());
            return new_stats;
        }
        
        stats.get(buyer_id).unwrap().clone()
    }
    
    // Record VM creation
    pub async fn record_vm_creation(&self, vm_id: &str, job_id: u64, buyer_id: &str, vcpu_count: u32, mem_size_mib: u32) {
        let mut stats = self.stats.lock().await;
        
        // Get or create buyer stats
        if !stats.contains_key(buyer_id) {
            let new_stats = BuyerSessionStats {
                buyer_id: buyer_id.to_string(),
                session_start: Utc::now(),
                active_vms: 0,
                total_vms_spawned: 0,
                total_earnings: 0.0,
                countries: HashMap::new(),
                transactions: Vec::new(),
            };
            
            stats.insert(buyer_id.to_string(), new_stats);
        }
        
        // Get buyer stats
        let buyer_stats = stats.get_mut(buyer_id).unwrap();
        
        // Generate random country info for simulation
        let country_info = self.generate_random_country();
        
        // Calculate earnings based on VM specs
        let hourly_rate = (vcpu_count as f64 * 0.05) + (mem_size_mib as f64 * 0.0001);
        
        // Create VM transaction
        let transaction = VmTransaction {
            vm_id: vm_id.to_string(),
            job_id,
            buyer_id: buyer_id.to_string(),
            customer_country: country_info.clone(),
            vcpu_count,
            mem_size_mib,
            created_at: Utc::now(),
            terminated_at: None,
            earnings: 0.0, // Will be updated when VM is terminated
            status: "running".to_string(),
        };
        
        // Update buyer stats
        buyer_stats.active_vms += 1;
        buyer_stats.total_vms_spawned += 1;
        buyer_stats.transactions.push(transaction);
        
        // Update country stats
        if let Some(country) = buyer_stats.countries.get_mut(&country_info.country_code) {
            country.vm_count += 1;
        } else {
            buyer_stats.countries.insert(country_info.country_code.clone(), country_info);
        }
        
        info!("{} {} {} {} {} {}", 
            "VM".green(), 
            vm_id.bright_yellow(), 
            "created for buyer".green(), 
            buyer_id.bright_cyan(), 
            "from".green(), 
            buyer_stats.countries.get(&transaction.customer_country.country_code).unwrap().flag_emoji.bright_white()
        );
    }
    
    // Record VM termination
    pub async fn record_vm_termination(&self, vm_id: &str, buyer_id: &str) {
        let mut stats = self.stats.lock().await;
        
        // Check if buyer exists
        if !stats.contains_key(buyer_id) {
            debug!("Buyer {} not found in stats", buyer_id);
            return;
        }
        
        // Get buyer stats
        let buyer_stats = stats.get_mut(buyer_id).unwrap();
        
        // Find VM transaction
        for transaction in &mut buyer_stats.transactions {
            if transaction.vm_id == vm_id {
                // Update transaction
                transaction.terminated_at = Some(Utc::now());
                transaction.status = "terminated".to_string();
                
                // Calculate runtime in hours
                let runtime_hours = (Utc::now() - transaction.created_at).num_seconds() as f64 / 3600.0;
                
                // Calculate earnings
                let hourly_rate = (transaction.vcpu_count as f64 * 0.05) + (transaction.mem_size_mib as f64 * 0.0001);
                let earnings = hourly_rate * runtime_hours;
                transaction.earnings = earnings;
                
                // Update buyer stats
                buyer_stats.active_vms -= 1;
                buyer_stats.total_earnings += earnings;
                
                // Update country stats
                if let Some(country) = buyer_stats.countries.get_mut(&transaction.customer_country.country_code) {
                    country.earnings += earnings;
                }
                
                info!("{} {} {} {} {} ${:.2} {} {}", 
                    "VM".yellow(), 
                    vm_id.bright_yellow(), 
                    "terminated for buyer".yellow(), 
                    buyer_id.bright_cyan(), 
                    "earnings:".green(),
                    earnings,
                    "from".yellow(),
                    transaction.customer_country.flag_emoji.bright_white()
                );
                
                break;
            }
        }
    }
    
    // Get buyer statistics
    pub async fn get_buyer_stats(&self, buyer_id: &str) -> Option<BuyerSessionStats> {
        let stats = self.stats.lock().await;
        stats.get(buyer_id).cloned()
    }
    
    // Get all buyer statistics
    pub async fn get_all_buyer_stats(&self) -> HashMap<String, BuyerSessionStats> {
        let stats = self.stats.lock().await;
        stats.clone()
    }
    
    // Generate random country info for simulation
    fn generate_random_country(&self) -> CountryInfo {
        let countries = vec![
            ("US", "United States", "🇺🇸"),
            ("GB", "United Kingdom", "🇬🇧"),
            ("DE", "Germany", "🇩🇪"),
            ("FR", "France", "🇫🇷"),
            ("JP", "Japan", "🇯🇵"),
            ("CN", "China", "🇨🇳"),
            ("IN", "India", "🇮🇳"),
            ("BR", "Brazil", "🇧🇷"),
            ("AU", "Australia", "🇦🇺"),
            ("CA", "Canada", "🇨🇦"),
            ("RU", "Russia", "🇷🇺"),
            ("KR", "South Korea", "🇰🇷"),
            ("IT", "Italy", "🇮🇹"),
            ("ES", "Spain", "🇪🇸"),
            ("NL", "Netherlands", "🇳🇱"),
            ("SE", "Sweden", "🇸🇪"),
            ("SG", "Singapore", "🇸🇬"),
            ("AE", "United Arab Emirates", "🇦🇪"),
            ("CH", "Switzerland", "🇨🇭"),
            ("NO", "Norway", "🇳🇴"),
        ];
        
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..countries.len());
        let (code, name, flag) = countries[index];
        
        CountryInfo {
            country_code: code.to_string(),
            country_name: name.to_string(),
            flag_emoji: flag.to_string(),
            vm_count: 1,
            earnings: 0.0,
        }
    }
}

// Format buyer statistics as a pretty string
pub fn format_buyer_stats(stats: &BuyerSessionStats) -> String {
    let mut result = String::new();
    
    // Format session info
    result.push_str(&format!("🧑‍💻 Buyer: {}\n", stats.buyer_id.bright_cyan()));
    result.push_str(&format!("⏱️ Session started: {}\n", stats.session_start.format("%Y-%m-%d %H:%M:%S UTC")));
    result.push_str(&format!("🔄 Active VMs: {}\n", stats.active_vms.to_string().bright_yellow()));
    result.push_str(&format!("📊 Total VMs spawned: {}\n", stats.total_vms_spawned.to_string().bright_yellow()));
    result.push_str(&format!("💰 Total earnings: ${:.2}\n\n", stats.total_earnings));
    
    // Format country info
    result.push_str("🌎 Customer Countries:\n");
    
    // Sort countries by VM count
    let mut countries: Vec<&CountryInfo> = stats.countries.values().collect();
    countries.sort_by(|a, b| b.vm_count.cmp(&a.vm_count));
    
    for country in countries {
        result.push_str(&format!("  {} {} - {} VMs, ${:.2} earnings\n", 
            country.flag_emoji,
            country.country_name,
            country.vm_count.to_string().bright_yellow(),
            country.earnings
        ));
    }
    
    // Format recent transactions
    result.push_str("\n🔄 Recent Transactions:\n");
    
    // Get the 5 most recent transactions
    let mut transactions = stats.transactions.clone();
    transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let recent_transactions = transactions.iter().take(5);
    
    for tx in recent_transactions {
        let status_str = match tx.status.as_str() {
            "running" => "Running".bright_green(),
            "terminated" => "Terminated".bright_red(),
            _ => tx.status.normal(),
        };
        
        result.push_str(&format!("  VM {} - Job {} - {} - {} - ${:.2}\n", 
            tx.vm_id.bright_yellow(),
            tx.job_id,
            tx.customer_country.flag_emoji,
            status_str,
            tx.earnings
        ));
    }
    
    result
}

