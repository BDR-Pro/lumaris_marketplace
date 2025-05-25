use eframe::{egui, App, CreationContext, NativeOptions};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod stats_sender;
use stats_sender::StatsSender;

// Enum to represent the current view
enum View {
    RoleSelection,
    BuyerDashboard,
    SellerDashboard,
}

// Enum to represent the user role
#[derive(Clone, PartialEq)]
enum UserRole {
    Buyer,
    Seller,
}

// Buyer statistics
#[derive(Default, Clone, Serialize, Deserialize)]
struct BuyerStats {
    pub buyer_id: String,
    pub active_vms: u32,
    pub total_vms_spawned: u32,
    pub total_earnings: f64,
    pub countries: HashMap<String, CountryInfo>,
}

// Country information
#[derive(Default, Clone, Serialize, Deserialize)]
struct CountryInfo {
    pub country_code: String,
    pub country_name: String,
    pub flag_emoji: String,
    pub vm_count: u32,
    pub earnings: f64,
}

// Seller statistics
#[derive(Default, Clone, Serialize, Deserialize)]
struct SellerStats {
    pub seller_id: String,
    pub active_nodes: u32,
    pub total_nodes: u32,
    pub active_vms: u32,
    pub total_vms_hosted: u32,
    pub total_earnings: f64,
    pub total_cpu_cores: f32,
    pub total_memory_mb: u64,
    pub nodes: HashMap<String, NodeInfo>,
}

// Node information
#[derive(Default, Clone, Serialize, Deserialize)]
struct NodeInfo {
    pub node_id: String,
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub vm_count: u32,
    pub earnings: f64,
    pub uptime_hours: f64,
    pub reliability_score: f32,
}

// VM creation parameters
#[derive(Default)]
struct VmCreationParams {
    pub job_id: String,
    pub vcpu_count: String,
    pub mem_size_mib: String,
}

// Node registration parameters
#[derive(Default)]
struct NodeRegistrationParams {
    pub cpu_cores: String,
    pub memory_mb: String,
}

// Main application state
#[derive(Default)]
struct LumarisMarketplace {
    view: View,
    user_role: Option<UserRole>,
    user_id: String,
    server_url: String,
    buyer_stats: BuyerStats,
    seller_stats: SellerStats,
    vm_params: VmCreationParams,
    node_params: NodeRegistrationParams,
    stats_sender: Option<StatsSender>,
    status_message: String,
}

impl App for LumarisMarketplace {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        match self.view {
            View::RoleSelection => self.show_role_selection(ctx),
            View::BuyerDashboard => self.show_buyer_dashboard(ctx),
            View::SellerDashboard => self.show_seller_dashboard(ctx),
        }

        ctx.request_repaint_after(Duration::from_millis(1000));
    }
}

impl LumarisMarketplace {
    // Create a new instance with default values
    fn new() -> Self {
        Self {
            view: View::RoleSelection,
            user_role: None,
            user_id: format!("user-{}", chrono::Utc::now().timestamp()),
            server_url: "ws://127.0.0.1:3030/ws".to_string(),
            buyer_stats: BuyerStats::default(),
            seller_stats: SellerStats::default(),
            vm_params: VmCreationParams::default(),
            node_params: NodeRegistrationParams::default(),
            stats_sender: None,
            status_message: String::new(),
        }
    }

    // Show the role selection view
    fn show_role_selection(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(egui::RichText::new("🌐 Lumaris Marketplace").size(32.0).strong());
                ui.add_space(20.0);

                ui.group(|ui| {
                    ui.style_mut().spacing.item_spacing = egui::vec2(10.0, 15.0);

                    ui.label(egui::RichText::new("Select Your Role").size(24.0));
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("User ID:");
                        ui.text_edit_singleline(&mut self.user_id);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Server URL:");
                        ui.text_edit_singleline(&mut self.server_url);
                    });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new("🛒 Enter as Buyer").size(18.0)).clicked() {
                            self.user_role = Some(UserRole::Buyer);
                            self.buyer_stats.buyer_id = self.user_id.clone();
                            self.view = View::BuyerDashboard;
                            
                            // Initialize stats sender for buyer
                            self.stats_sender = Some(StatsSender::new(
                                &self.server_url,
                                &self.user_id,
                                5000,
                                "buyer".to_string(),
                            ));
                            
                            if let Some(sender) = &self.stats_sender {
                                sender.start();
                            }
                        }

                        if ui.button(egui::RichText::new("🖥️ Enter as Seller").size(18.0)).clicked() {
                            self.user_role = Some(UserRole::Seller);
                            self.seller_stats.seller_id = self.user_id.clone();
                            self.view = View::SellerDashboard;
                            
                            // Initialize stats sender for seller
                            self.stats_sender = Some(StatsSender::new(
                                &self.server_url,
                                &self.user_id,
                                5000,
                                "seller".to_string(),
                            ));
                            
                            if let Some(sender) = &self.stats_sender {
                                sender.start();
                            }
                        }
                    });
                });
            });
        });
    }

    // Show the buyer dashboard
    fn show_buyer_dashboard(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(egui::RichText::new("🛒 Buyer Dashboard").size(32.0).strong());
                ui.label(egui::RichText::new(format!("Buyer ID: {}", self.user_id)).size(16.0));
                ui.add_space(20.0);

                // VM Creation Panel
                ui.group(|ui| {
                    ui.heading(egui::RichText::new("Create VM").size(20.0));
                    
                    ui.horizontal(|ui| {
                        ui.label("Job ID:");
                        ui.text_edit_singleline(&mut self.vm_params.job_id);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("vCPU Count:");
                        ui.text_edit_singleline(&mut self.vm_params.vcpu_count);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Memory (MiB):");
                        ui.text_edit_singleline(&mut self.vm_params.mem_size_mib);
                    });
                    
                    if ui.button("Create VM").clicked() {
                        self.create_vm();
                    }
                });
                
                ui.add_space(10.0);
                
                // Statistics Panel
                ui.group(|ui| {
                    ui.heading(egui::RichText::new("Buyer Statistics").size(20.0));
                    
                    if ui.button("Refresh Statistics").clicked() {
                        self.get_buyer_stats();
                    }
                    
                    ui.add_space(10.0);
                    
                    ui.label(egui::RichText::new(format!("Active VMs: {}", self.buyer_stats.active_vms)).size(16.0));
                    ui.label(egui::RichText::new(format!("Total VMs Spawned: {}", self.buyer_stats.total_vms_spawned)).size(16.0));
                    ui.label(egui::RichText::new(format!("Total Earnings: ${:.2}", self.buyer_stats.total_earnings)).size(16.0));
                    
                    ui.add_space(10.0);
                    
                    ui.label(egui::RichText::new("Customer Countries:").size(16.0));
                    for (_, country) in &self.buyer_stats.countries {
                        ui.label(format!("{} {} - {} VMs, ${:.2} earnings", 
                            country.flag_emoji, 
                            country.country_name,
                            country.vm_count,
                            country.earnings
                        ));
                    }
                });
                
                ui.add_space(10.0);
                
                // Status message
                if !self.status_message.is_empty() {
                    ui.label(egui::RichText::new(&self.status_message).size(14.0).color(egui::Color32::YELLOW));
                }
                
                // Back button
                if ui.button("Back to Role Selection").clicked() {
                    self.view = View::RoleSelection;
                    self.status_message.clear();
                }
            });
        });
    }

    // Show the seller dashboard
    fn show_seller_dashboard(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(egui::RichText::new("🖥️ Seller Dashboard").size(32.0).strong());
                ui.label(egui::RichText::new(format!("Seller ID: {}", self.user_id)).size(16.0));
                ui.add_space(20.0);

                // Node Registration Panel
                ui.group(|ui| {
                    ui.heading(egui::RichText::new("Register Node").size(20.0));
                    
                    ui.horizontal(|ui| {
                        ui.label("CPU Cores:");
                        ui.text_edit_singleline(&mut self.node_params.cpu_cores);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Memory (MB):");
                        ui.text_edit_singleline(&mut self.node_params.memory_mb);
                    });
                    
                    if ui.button("Register Node").clicked() {
                        self.register_node();
                    }
                });
                
                ui.add_space(10.0);
                
                // Statistics Panel
                ui.group(|ui| {
                    ui.heading(egui::RichText::new("Seller Statistics").size(20.0));
                    
                    if ui.button("Refresh Statistics").clicked() {
                        self.get_seller_stats();
                    }
                    
                    ui.add_space(10.0);
                    
                    ui.label(egui::RichText::new(format!("Active Nodes: {}/{}", 
                        self.seller_stats.active_nodes, 
                        self.seller_stats.total_nodes
                    )).size(16.0));
                    
                    ui.label(egui::RichText::new(format!("Active VMs: {}/{}", 
                        self.seller_stats.active_vms, 
                        self.seller_stats.total_vms_hosted
                    )).size(16.0));
                    
                    ui.label(egui::RichText::new(format!("Total Earnings: ${:.2}", 
                        self.seller_stats.total_earnings
                    )).size(16.0));
                    
                    ui.label(egui::RichText::new(format!("Total Resources: {} CPU cores, {} MB RAM", 
                        self.seller_stats.total_cpu_cores, 
                        self.seller_stats.total_memory_mb
                    )).size(16.0));
                    
                    ui.add_space(10.0);
                    
                    ui.label(egui::RichText::new("Nodes:").size(16.0));
                    for (_, node) in &self.seller_stats.nodes {
                        ui.label(format!("Node {} - {} VMs, {} CPU cores, {} MB RAM, ${:.2} earnings", 
                            node.node_id,
                            node.vm_count,
                            node.cpu_cores,
                            node.memory_mb,
                            node.earnings
                        ));
                    }
                });
                
                ui.add_space(10.0);
                
                // Status message
                if !self.status_message.is_empty() {
                    ui.label(egui::RichText::new(&self.status_message).size(14.0).color(egui::Color32::YELLOW));
                }
                
                // Back button
                if ui.button("Back to Role Selection").clicked() {
                    self.view = View::RoleSelection;
                    self.status_message.clear();
                }
            });
        });
    }

    // Create a VM
    fn create_vm(&mut self) {
        if let Some(stats_sender) = &self.stats_sender {
            // Validate inputs
            let job_id = match self.vm_params.job_id.parse::<u64>() {
                Ok(id) => id,
                Err(_) => {
                    self.status_message = "Invalid Job ID. Please enter a number.".to_string();
                    return;
                }
            };
            
            let vcpu_count = match self.vm_params.vcpu_count.parse::<u32>() {
                Ok(count) => count,
                Err(_) => {
                    self.status_message = "Invalid vCPU count. Please enter a number.".to_string();
                    return;
                }
            };
            
            let mem_size_mib = match self.vm_params.mem_size_mib.parse::<u32>() {
                Ok(size) => size,
                Err(_) => {
                    self.status_message = "Invalid memory size. Please enter a number.".to_string();
                    return;
                }
            };
            
            // Send VM creation request
            stats_sender.send_create_vm(job_id, vcpu_count, mem_size_mib);
            self.status_message = "VM creation request sent.".to_string();
        } else {
            self.status_message = "Not connected to server.".to_string();
        }
    }

    // Register a node
    fn register_node(&mut self) {
        if let Some(stats_sender) = &self.stats_sender {
            // Validate inputs
            let cpu_cores = match self.node_params.cpu_cores.parse::<f32>() {
                Ok(cores) => cores,
                Err(_) => {
                    self.status_message = "Invalid CPU cores. Please enter a number.".to_string();
                    return;
                }
            };
            
            let memory_mb = match self.node_params.memory_mb.parse::<u64>() {
                Ok(mem) => mem,
                Err(_) => {
                    self.status_message = "Invalid memory size. Please enter a number.".to_string();
                    return;
                }
            };
            
            // Send node registration request
            stats_sender.send_node_registration(cpu_cores, memory_mb);
            self.status_message = "Node registration request sent.".to_string();
        } else {
            self.status_message = "Not connected to server.".to_string();
        }
    }

    // Get buyer statistics
    fn get_buyer_stats(&mut self) {
        if let Some(stats_sender) = &self.stats_sender {
            stats_sender.send_get_buyer_stats();
            self.status_message = "Buyer statistics request sent.".to_string();
        } else {
            self.status_message = "Not connected to server.".to_string();
        }
    }

    // Get seller statistics
    fn get_seller_stats(&mut self) {
        if let Some(stats_sender) = &self.stats_sender {
            stats_sender.send_get_seller_stats();
            self.status_message = "Seller statistics request sent.".to_string();
        } else {
            self.status_message = "Not connected to server.".to_string();
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    let options = NativeOptions {
        drag_and_drop_support: false,
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Lumaris Marketplace",
        options,
        Box::new(|_cc: &CreationContext| Box::new(LumarisMarketplace::new())),
    )
}

