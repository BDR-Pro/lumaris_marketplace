use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use tokio::sync::broadcast;

// Data structures for the matchmaking system

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub node_id: String,
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub available: bool,
    pub reliability_score: f32,
    pub last_updated: u64,  // Unix timestamp
}

// Alias for clarity
pub type Node = NodeCapabilities;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobRequirements {
    pub job_id: u64,
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub expected_duration_sec: u64,
    pub priority: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Matching,
    Assigned,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub requirements: JobRequirements,
    pub status: JobStatus,
    pub assigned_node: Option<String>,
    pub submitted_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

// Messages for communication with matchmaker
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MatchmakerMessage {
    NodeAvailable(NodeCapabilities),
    NodeUnavailable(String), // node_id
    JobSubmitted(Job),
    JobStatusUpdate(u64, JobStatus), // job_id, new status
}

pub struct MatchMaker {
    nodes: HashMap<String, NodeCapabilities>,
    job_queue: VecDeque<Job>,
    running_jobs: HashMap<u64, Job>,
    assigned_jobs: HashMap<u64, Job>,
    next_job_id: u64,
    tx: broadcast::Sender<String>,
}

impl MatchMaker {
    pub fn new(tx: broadcast::Sender<String>) -> Self {
        Self {
            nodes: HashMap::new(),
            job_queue: VecDeque::new(),
            running_jobs: HashMap::new(),
            assigned_jobs: HashMap::new(),
            next_job_id: 1,
            tx,
        }
    }

    // Register or update node capabilities
    pub fn update_node(&mut self, capabilities: NodeCapabilities) {
        println!("🖥️ Node {} updated: {:.1} cores, {} MB memory", 
            capabilities.node_id, capabilities.cpu_cores, capabilities.memory_mb);
        
        self.nodes.insert(capabilities.node_id.clone(), capabilities);
        
        // Try to match waiting jobs
        self.match_jobs();
    }

    // Mark a node as unavailable
    pub fn remove_node(&mut self, node_id: &str) {
        println!("❌ Node {} removed from matchmaking pool", node_id);
        self.nodes.remove(node_id);
        
        // Check if we need to reassign jobs
        self.handle_node_failure(node_id);
    }

    // Submit a new job for matching
    pub fn submit_job(&mut self, mut job: Job) -> u64 {
        job.id = self.next_job_id;
        self.next_job_id += 1;
        
        job.status = JobStatus::Queued;
        println!("📋 Job {} added to queue", job.id);
        
        self.job_queue.push_back(job.clone());
        
        // Try immediate matching
        self.match_jobs();
        
        job.id
    }

    // Core matching algorithm
    fn match_jobs(&mut self) {
        if self.job_queue.is_empty() || self.nodes.is_empty() {
            return;
        }
        
        println!("🔄 Running matching algorithm. Jobs in queue: {}", self.job_queue.len());
        
        // Sort nodes by available capacity and reliability
        let mut available_nodes: Vec<NodeCapabilities> = self.nodes.values()
            .filter(|n| n.available)
            .cloned()
            .collect();
        
        available_nodes.sort_by(|a, b| {
            let a_score = a.cpu_cores * a.reliability_score;
            let b_score = b.cpu_cores * b.reliability_score;
            b_score.partial_cmp(&a_score).unwrap()
        });
        
        // Process each job in the queue
        let mut matched_indices = Vec::new();
        
        for (job_index, job) in self.job_queue.iter().enumerate() {
            // Find a suitable node
            if let Some(best_node_index) = self.find_best_node(job, &available_nodes) {
                let best_node = &available_nodes[best_node_index];
                println!("✅ Matched Job {} to Node {}", job.id, best_node.node_id);
                
                // Mark this job for assignment
                matched_indices.push((job_index, best_node.node_id.clone()));
                
                // Update our working copy of available nodes
                let mut updated_caps = available_nodes[best_node_index].clone();
                updated_caps.cpu_cores -= job.requirements.cpu_cores;
                updated_caps.memory_mb -= job.requirements.memory_mb;
                
                // If node is now at capacity, remove it from available pool
                if updated_caps.cpu_cores < 0.5 || updated_caps.memory_mb < 512 {
                    available_nodes.remove(best_node_index);
                } else {
                    // Otherwise update its capacity
                    available_nodes[best_node_index] = updated_caps;
                }
            }
        }
        
        // Now actually assign the matched jobs
        for (job_index, node_id) in matched_indices.iter().rev() {
            if let Some(mut job) = self.job_queue.remove(*job_index) {
                job.status = JobStatus::Assigned;
                
                // Dispatch job to node
                println!("📤 Dispatching Job {} to Node {}", job.id, node_id);
                
                // In a full implementation, you would send the job to the node here
                
                // Update job status
                job.status = JobStatus::Assigned;
                
                // Store the job in the assigned jobs map
                self.assigned_jobs.insert(job.id, job);
            }
        }
    }
    
    // Find the best node for a job based on requirements and scoring
    fn find_best_node(&self, job: &Job, available_nodes: &[NodeCapabilities]) -> Option<usize> {
        available_nodes.iter()
            .enumerate()
            .filter(|(_, node)| {
                // Basic filtering: enough CPU and memory
                node.cpu_cores >= job.requirements.cpu_cores &&
                node.memory_mb >= job.requirements.memory_mb
            })
            .max_by(|(_, a), (_, b)| {
                // Scoring: balance between utilization and reliability
                let a_score = a.reliability_score * (a.cpu_cores / job.requirements.cpu_cores);
                let b_score = b.reliability_score * (b.cpu_cores / job.requirements.cpu_cores);
                a_score.partial_cmp(&b_score).unwrap()
            })
            .map(|(index, _)| index)
    }
    
    // Handle node failure: requeue jobs that were assigned to the failed node
    fn handle_node_failure(&mut self, node_id: &str) {
        let failed_jobs: Vec<u64> = self.running_jobs.iter()
            .filter_map(|(job_id, job)| {
                if let Some(assigned_node) = &job.assigned_node {
                    if assigned_node == node_id {
                        return Some(*job_id);
                    }
                }
                None
            })
            .collect();
        
        for job_id in failed_jobs {
            if let Some(mut job) = self.running_jobs.remove(&job_id) {
                println!("⚠️ Re-queueing Job {} due to Node {} failure", job_id, node_id);
                job.status = JobStatus::Queued;
                job.assigned_node = None;
                self.job_queue.push_front(job);
            }
        }
    }
    
    // Update job status
    pub fn update_job_status(&mut self, job_id: u64, status: String) -> Result<(), String> {
        // Find the job in running jobs
        if let Some(job) = self.running_jobs.get_mut(&job_id) {
            // Update the status
            match status.as_str() {
                "running" => job.status = JobStatus::Running,
                "completed" => job.status = JobStatus::Completed,
                "failed" => job.status = JobStatus::Failed,
                _ => return Err(format!("Invalid status: {}", status)),
            }
            
            // Send a status update message
            let status_json = serde_json::json!({
                "type": "job_status_update",
                "job_id": job_id,
                "status": status
            }).to_string();
            
            let _ = self.tx.send(status_json);
            
            Ok(())
        } else {
            Err(format!("Job {} not found", job_id))
        }
    }
    
    pub fn get_node_by_id(&self, node_id: &str) -> Option<&NodeCapabilities> {
        self.nodes.get(node_id)
    }
    
    pub fn get_node_by_id_mut(&mut self, node_id: &str) -> Option<&mut NodeCapabilities> {
        self.nodes.get_mut(node_id)
    }
    
    // Add a setter for tx
    pub fn set_tx(&mut self, tx: broadcast::Sender<String>) {
        self.tx = tx;
    }
    
    // Add update_node_availability method
    pub fn update_node_availability(&mut self, node_id: String, available: bool) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.available = available;
            println!("Node {} availability updated to {}", node_id, available);
        }
    }
}

// Thread-safe matchmaker for use in async context
pub type SharedMatchMaker = Arc<Mutex<MatchMaker>>;

// Helper function to create a new shared matchmaker
pub fn create_matchmaker() -> (SharedMatchMaker, broadcast::Receiver<String>) {
    let (tx, rx) = broadcast::channel(100);
    let matchmaker = MatchMaker::new(tx);
    let shared_matchmaker = Arc::new(Mutex::new(matchmaker));
    
    (shared_matchmaker, rx)
}
