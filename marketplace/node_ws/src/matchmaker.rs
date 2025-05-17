
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
    next_job_id: u64,
    tx: broadcast::Sender<MatchmakerMessage>,
}

impl MatchMaker {
    pub fn new(tx: broadcast::Sender<MatchmakerMessage>) -> Self {
        Self {
            nodes: HashMap::new(),
            job_queue: VecDeque::new(),
            running_jobs: HashMap::new(),
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
        let mut available_nodes: Vec<&NodeCapabilities> = self.nodes.values()
            .filter(|n| n.available)
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
            if let Some(best_node) = self.find_best_node(job, &available_nodes) {
                println!("✅ Matched Job {} to Node {}", job.id, best_node.node_id);
                
                // Mark this job for assignment
                matched_indices.push((job_index, best_node.node_id.clone()));
                
                // Update our working copy of available nodes
                if let Some(node_index) = available_nodes.iter().position(|n| n.node_id == best_node.node_id) {
                    let mut updated_caps = best_node.clone();
                    updated_caps.cpu_cores -= job.requirements.cpu_cores;
                    updated_caps.memory_mb -= job.requirements.memory_mb;
                    
                    // If node is now at capacity, remove it from available pool
                    if updated_caps.cpu_cores < 0.5 || updated_caps.memory_mb < 512 {
                        available_nodes.remove(node_index);
                    } else {
                        // Otherwise update its capacity
                        available_nodes[node_index] = &updated_caps;
                    }
                }
            }
        }
        
        // Now actually assign the matched jobs
        for (job_index, node_id) in matched_indices.iter().rev() {
            if let Some(mut job) = self.job_queue.remove(*job_index) {
                job.status = JobStatus::Assigned;
                job.assigned_node = Some(node_id.clone());
                
                // In a full implementation, you would send the job to the node here
                println!("📤 Dispatching Job {} to Node {}", job.id, node_id);
                
                // Update job status and move to running jobs map
                let _ = self.tx.send(MatchmakerMessage::JobStatusUpdate(job.id, job.status.clone()));
                self.running_jobs.insert(job.id, job);
            }
        }
    }
    
    // Find the best node for a job based on requirements and scoring
    fn find_best_node<'a>(&self, job: &Job, available_nodes: &[&'a NodeCapabilities]) -> Option<&'a NodeCapabilities> {
        available_nodes.iter()
            .filter(|node| {
                // Basic filtering: enough CPU and memory
                node.cpu_cores >= job.requirements.cpu_cores &&
                node.memory_mb >= job.requirements.memory_mb
            })
            .max_by(|a, b| {
                // Scoring: balance between utilization and reliability
                let a_score = a.reliability_score * (a.cpu_cores / job.requirements.cpu_cores);
                let b_score = b.reliability_score * (b.cpu_cores / job.requirements.cpu_cores);
                a_score.partial_cmp(&b_score).unwrap()
            })
            .copied()
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
    pub fn update_job_status(&mut self, job_id: u64, status: JobStatus) {
        if let Some(job) = self.running_jobs.get_mut(&job_id) {
            job.status = status.clone();
            println!("📝 Job {} status updated to {:?}", job_id, status);
            
            match status {
                JobStatus::Completed | JobStatus::Failed => {
                    // Remove from running jobs if complete or failed
                    if let Some(mut job) = self.running_jobs.remove(&job_id) {
                        job.completed_at = Some(chrono::Utc::now().timestamp() as u64);
                        
                        // In a full implementation, you might store this in a database
                        println!("🏁 Job {} finished with status {:?}", job_id, status);
                    }
                }
                _ => {}
            }
            
            let _ = self.tx.send(MatchmakerMessage::JobStatusUpdate(job_id, status));
        }
    }
}

// Thread-safe matchmaker for use in async context
pub type SharedMatchMaker = Arc<Mutex<MatchMaker>>;

// Helper function to create a new shared matchmaker
pub fn create_matchmaker() -> (SharedMatchMaker, broadcast::Receiver<MatchmakerMessage>) {
    let (tx, rx) = broadcast::channel(100);
    let matchmaker = Arc::new(Mutex::new(MatchMaker::new(tx)));
    (matchmaker, rx)
}zx