// File: marketplace/node_ws/src/job_scheduler.rs

use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use crate::matchmaker::{SharedMatchMaker, JobStatus};
use distributed_engine::{
    DistributedJobManager, JobPayload, JobChunk, JobResult
};

// Job scheduler is responsible for:
// 1. Receiving job submissions
// 2. Using the distributed engine to split jobs into chunks
// 3. Using the matchmaker to assign chunks to nodes
// 4. Tracking job progress and aggregating results

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: u64,
    pub status: String,
    pub chunks_total: usize,
    pub chunks_completed: usize,
    pub chunks_failed: usize,
    pub submitted_at: u64,
    pub completed_at: Option<u64>,
    pub priority: u8,
    pub user_id: String,
}

pub struct JobScheduler {
    matchmaker: SharedMatchMaker,
    job_manager: Arc<Mutex<DistributedJobManager>>,
    scheduled_jobs: HashMap<u64, ScheduledJob>,
    next_job_id: u64,
}

impl JobScheduler {
    pub fn new(matchmaker: SharedMatchMaker) -> Self {
        Self {
            matchmaker,
            job_manager: Arc::new(Mutex::new(DistributedJobManager::default())),
            scheduled_jobs: HashMap::new(),
            next_job_id: 1,
        }
    }
    
    pub fn submit_job(&mut self, payload: JobPayload, user_id: String, priority: u8) -> u64 {
        let job_id = self.next_job_id;
        self.next_job_id += 1;
        
        // Use the distributed engine to split the job into chunks
        let chunks = {
            let mut job_manager = self.job_manager.lock().unwrap();
            job_manager.prepare_job(job_id, payload)
        };
        
        // Track job in our scheduler
        let scheduled_job = ScheduledJob {
            id: job_id,
            status: "scheduled".to_string(),
            chunks_total: chunks.len(),
            chunks_completed: 0,
            chunks_failed: 0,
            submitted_at: chrono::Utc::now().timestamp() as u64,
            completed_at: None,
            priority,
            user_id,
        };
        
        self.scheduled_jobs.insert(job_id, scheduled_job);
        
        // Process chunks that don't have dependencies
        let initial_chunks: Vec<_> = chunks.iter()
            .filter(|chunk| chunk.dependencies.is_empty())
            .collect();
        
        println!("Job {} split into {} chunks, {} ready for execution", 
            job_id, chunks.len(), initial_chunks.len());
        
        // Submit initial chunks to matchmaker
        self.submit_chunks_to_matchmaker(&initial_chunks);
        
        job_id
    }
    
    fn submit_chunks_to_matchmaker(&mut self, chunks: &[&JobChunk]) {
        for chunk in chunks {
            // Convert chunk to matchmaker job
            // In a real implementation, this would include proper requirements calculation
            let job_requirements = crate::matchmaker::JobRequirements {
                job_id: chunk.chunk_id,
                cpu_cores: 1.0, // Default for now
                memory_mb: 256, // Default for now
                expected_duration_sec: chunk.estimated_work_units * 10, // Rough estimate
                priority: 1,    // Default priority
            };
            
            let matchmaker_job = crate::matchmaker::Job {
                id: 0, // Will be assigned by matchmaker
                requirements: job_requirements,
                status: crate::matchmaker::JobStatus::Queued,
                assigned_node: None,
                submitted_at: chrono::Utc::now().timestamp() as u64,
                started_at: None,
                completed_at: None,
            };
            
            // Submit to matchmaker
            let mm_job_id = {
                let mut mm = self.matchmaker.lock().unwrap();
                mm.submit_job(matchmaker_job)
            };
            
            println!("Chunk {} of job {} submitted to matchmaker as job {}", 
                chunk.chunk_id, chunk.parent_job_id, mm_job_id);
        }
    }
    
    pub fn update_chunk_status(&mut self, chunk_id: u64, parent_job_id: u64, status: &str) {
        if let Some(job) = self.scheduled_jobs.get_mut(&parent_job_id) {
            match status {
                "completed" => {
                    job.chunks_completed += 1;
                }
                "failed" => {
                    job.chunks_failed += 1;
                }
                _ => {}
            }
            
            // Update overall job status
            if job.chunks_completed + job.chunks_failed == job.chunks_total {
                if job.chunks_failed > 0 {
                    job.status = "failed".to_string();
                } else {
                    job.status = "completed".to_string();
                }
                job.completed_at = Some(chrono::Utc::now().timestamp() as u64);
                
                println!("Job {} completed with status: {}", parent_job_id, job.status);
            }
        }
    }
    
    pub fn record_chunk_result(&mut self, result: JobResult) {
        // Record the result in the job manager
        let final_result = {
            let mut job_manager = self.job_manager.lock().unwrap();
            job_manager.record_result(result.clone())
        };
        
        // Update chunk status
        self.update_chunk_status(result.chunk_id, result.parent_job_id, 
                                if result.success { "completed" } else { "failed" });
        
        // If we got a final result, the job is complete
        if let Some(final_result) = final_result {
            // In a real implementation, store the final result and notify the user
            println!("Final result for job {}: success={}", 
                     final_result.parent_job_id, final_result.success);
        } else {
            // Check if we need to schedule more chunks
            self.schedule_dependent_chunks(result.parent_job_id, result.chunk_id);
        }
    }
    
    fn schedule_dependent_chunks(&mut self, job_id: u64, completed_chunk_id: u64) {
        // Get completed chunks for this job
        let completed_chunks: Vec<u64> = self.scheduled_jobs
            .get(&job_id)
            .map(|job| {
                // In a real implementation, we'd query all completed chunks
                // For now, just use the one we know completed
                vec![completed_chunk_id]
            })
            .unwrap_or_default();
        
        // Find chunks that are now ready to run
        let ready_chunks = {
            let job_manager = self.job_manager.lock().unwrap();
            job_manager.get_ready_chunks(job_id, &completed_chunks)
        };
        
        // Submit ready chunks to matchmaker
        if !ready_chunks.is_empty() {
            println!("{} additional chunks for job {} are now ready", 
                     ready_chunks.len(), job_id);
            self.submit_chunks_to_matchmaker(&ready_chunks.iter().collect::<Vec<_>>());
        }
    }
    
    pub fn get_job_status(&self, job_id: u64) -> Option<&ScheduledJob> {
        self.scheduled_jobs.get(&job_id)
    }
    
    pub fn get_all_jobs(&self) -> Vec<&ScheduledJob> {
        self.scheduled_jobs.values().collect()
    }
    
    pub fn cancel_job(&mut self, job_id: u64) -> bool {
        // Cancel in job manager
        {
            let mut job_manager = self.job_manager.lock().unwrap();
            job_manager.cancel_job(job_id);
        }
        
        // Update scheduler status
        if let Some(job) = self.scheduled_jobs.get_mut(&job_id) {
            job.status = "cancelled".to_string();
            job.completed_at = Some(chrono::Utc::now().timestamp() as u64);
            true
        } else {
            false
        }
    }
}

// Type for shared job scheduler
pub type SharedJobScheduler = Arc<Mutex<JobScheduler>>;

// Helper to create a new job scheduler
pub fn create_job_scheduler(matchmaker: SharedMatchMaker) -> SharedJobScheduler {
    Arc::new(Mutex::new(JobScheduler::new(matchmaker)))
}