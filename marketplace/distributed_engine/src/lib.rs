// File: marketplace/distributed_engine/src/lib.rs

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

pub mod error;
pub use error::{EngineError, Result};

#[cfg(test)]
mod tests;

// Define job chunking and distributed execution types

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobPayload {
    pub command: String,
    pub args: Vec<String>,
    pub input_data: Option<String>,
    pub env_vars: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobChunk {
    pub chunk_id: u64,
    pub parent_job_id: u64,
    pub payload: JobPayload,
    pub dependencies: Vec<u64>, // IDs of chunks this one depends on
    pub estimated_work_units: u64, // Used for load balancing
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobResult {
    pub chunk_id: u64,
    pub parent_job_id: u64,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub execution_stats: ExecutionStats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub start_time: u64,
    pub end_time: u64,
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: u64,
    pub error_count: u64,
}

// Core interfaces for the distributed engine

pub trait JobSplitter {
    /// Split a job into multiple chunks that can be executed in parallel
    fn split_job(&self, job_id: u64, payload: JobPayload) -> Vec<JobChunk>;
    
    /// Calculate dependencies between job chunks
    fn calculate_dependencies(&self, chunks: &mut Vec<JobChunk>);
}

pub trait ResultAggregator {
    /// Combine results from multiple chunks into a final job result
    fn aggregate_results(&self, results: Vec<JobResult>) -> JobResult;
    
    /// Check if all required chunks have completed
    fn is_job_complete(&self, job_id: u64, received_chunks: &[u64]) -> bool;
}

// Default implementations

pub struct DefaultJobSplitter {
    chunk_size_threshold: usize,
}

impl DefaultJobSplitter {
    pub fn new(chunk_size_threshold: usize) -> Self {
        Self { chunk_size_threshold }
    }
}

impl JobSplitter for DefaultJobSplitter {
    fn split_job(&self, job_id: u64, payload: JobPayload) -> Vec<JobChunk> {
        // If the payload has input data, split it by lines
        if let Some(data) = &payload.input_data {
            let lines: Vec<&str> = data.lines().collect();
            
            // Calculate number of chunks based on input size and threshold
            let num_chunks = lines.len().div_ceil(self.chunk_size_threshold);
            
            // Create chunks
            let mut chunks = Vec::with_capacity(num_chunks + 1); // +1 for aggregator
            
            for i in 0..num_chunks {
                let start = i * self.chunk_size_threshold;
                let end = std::cmp::min((i + 1) * self.chunk_size_threshold, lines.len());
                
                let chunk_data = lines[start..end].join("\n");
                
                let chunk_payload = JobPayload {
                    command: payload.command.clone(),
                    args: payload.args.clone(),
                    input_data: Some(chunk_data),
                    env_vars: payload.env_vars.clone(),
                };
                
                let chunk = JobChunk {
                    chunk_id: (i + 1) as u64,
                    parent_job_id: job_id,
                    payload: chunk_payload,
                    dependencies: Vec::new(),
                    estimated_work_units: (end - start) as u64,
                };
                
                chunks.push(chunk);
            }
            
            // Create an aggregator chunk that will combine results
            let aggregator_payload = JobPayload {
                command: format!("{}_aggregate", payload.command),
                args: payload.args.clone(),
                input_data: None,  // Aggregator doesn't need input data
                env_vars: payload.env_vars.clone(),
            };
            
            let aggregator_chunk = JobChunk {
                chunk_id: (num_chunks + 1) as u64,
                parent_job_id: job_id,
                payload: aggregator_payload,
                dependencies: (1..=num_chunks as u64).collect(), // Depends on all other chunks
                estimated_work_units: 10, // Small fixed cost for aggregation
            };
            
            chunks.push(aggregator_chunk);
            
            chunks
        } else {
            // If no input data, just create a single chunk
            vec![JobChunk {
                chunk_id: 1,
                parent_job_id: job_id,
                payload: payload.clone(),
                dependencies: Vec::new(),
                estimated_work_units: 100, // Default work units
            }]
        }
    }
    
    fn calculate_dependencies(&self, chunks: &mut Vec<JobChunk>) {
        // Simple linear dependencies by default - more complex DAGs would be configured here
        // This implementation just ensures the aggregator chunk depends on all worker chunks
        
        if chunks.len() <= 1 {
            return;
        }
        
        // Find the highest chunk_id (assumed to be the aggregator)
        if let Some(max_chunk_id) = chunks.iter().map(|c| c.chunk_id).max() {
            // Find all other chunk IDs
            let dependency_ids: Vec<u64> = chunks.iter()
                .filter(|c| c.chunk_id != max_chunk_id)
                .map(|c| c.chunk_id)
                .collect();
            
            // Update the dependencies for the max chunk
            for chunk in chunks.iter_mut() {
                if chunk.chunk_id == max_chunk_id {
                    chunk.dependencies = dependency_ids.clone();
                    break;
                }
            }
        }
    }
}

pub struct DefaultResultAggregator;

impl ResultAggregator for DefaultResultAggregator {
    fn aggregate_results(&self, results: Vec<JobResult>) -> JobResult {
        if results.is_empty() {
            return JobResult {
                chunk_id: 0,
                parent_job_id: 0,
                success: false,
                output: None,
                error: Some("No results to aggregate".to_string()),
                execution_stats: ExecutionStats {
                    start_time: 0,
                    end_time: 0,
                    cpu_usage_percent: 0.0,
                    memory_usage_mb: 0,
                    error_count: 1,
                },
            };
        }
        
        // Check if there's already an aggregator result (the final chunk)
        let parent_job_id = results[0].parent_job_id;
        let max_chunk_id = results.iter().map(|r| r.chunk_id).max().unwrap_or(0);
        
        // If the last chunk (aggregator) has a result, use that
        if let Some(aggregator_result) = results.iter().find(|r| r.chunk_id == max_chunk_id) {
            return aggregator_result.clone();
        }
        
        // Otherwise, combine results manually
        let all_success = results.iter().all(|r| r.success);
        
        // Combine outputs
        let combined_output = results.iter()
            .filter_map(|r| r.output.as_ref())
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        
        // Combine errors
        let errors: Vec<_> = results.iter()
            .filter_map(|r| r.error.as_ref())
            .map(|s| s.as_str())
            .collect();
        
        let combined_error = if errors.is_empty() {
            None
        } else {
            Some(errors.join("\n").to_string())
        };
        
        // Calculate aggregate stats
        let start_time = results.iter()
            .map(|r| r.execution_stats.start_time)
            .min()
            .unwrap_or(0);
        
        let end_time = results.iter()
            .map(|r| r.execution_stats.end_time)
            .max()
            .unwrap_or(0);
        
        let total_cpu = results.iter()
            .map(|r| r.execution_stats.cpu_usage_percent)
            .sum::<f32>();
        
        let max_memory = results.iter()
            .map(|r| r.execution_stats.memory_usage_mb)
            .max()
            .unwrap_or(0);
        
        let error_count = results.iter()
            .map(|r| r.execution_stats.error_count)
            .sum();
        
        JobResult {
            chunk_id: 0,  // 0 indicates an aggregated result
            parent_job_id,
            success: all_success,
            output: if combined_output.is_empty() { None } else { Some(combined_output) },
            error: combined_error,
            execution_stats: ExecutionStats {
                start_time,
                end_time,
                cpu_usage_percent: total_cpu / results.len() as f32,
                memory_usage_mb: max_memory,
                error_count,
            },
        }
    }
    
    fn is_job_complete(&self, _job_id: u64, received_chunks: &[u64]) -> bool {
        // In a real implementation, we'd check against expected chunks from a database
        // For this example, we'll assume we're complete if we have at least one result
        !received_chunks.is_empty()
    }
}

// Job execution coordination

pub struct DistributedJobManager {
    job_splitter: Box<dyn JobSplitter>,
    result_aggregator: Box<dyn ResultAggregator>,
    chunks_by_job: HashMap<u64, Vec<JobChunk>>,
    results_by_job: HashMap<u64, Vec<JobResult>>,
}

impl DistributedJobManager {
    pub fn new(
        job_splitter: Box<dyn JobSplitter>,
        result_aggregator: Box<dyn ResultAggregator>
    ) -> Self {
        Self {
            job_splitter,
            result_aggregator,
            chunks_by_job: HashMap::new(),
            results_by_job: HashMap::new(),
        }
    }
    
    /// Create a new DistributedJobManager with default implementations
    #[must_use]
    pub fn new_default() -> Self {
        Self::new(
            Box::new(DefaultJobSplitter::new(1000)),
            Box::new(DefaultResultAggregator)
        )
    }
    
    pub fn prepare_job(&mut self, job_id: u64, payload: JobPayload) -> Vec<JobChunk> {
        // Split the job into chunks
        let mut chunks = self.job_splitter.split_job(job_id, payload);
        
        // Calculate dependencies between chunks
        self.job_splitter.calculate_dependencies(&mut chunks);
        
        // Store chunks for later reference
        self.chunks_by_job.insert(job_id, chunks.clone());
        
        chunks
    }
    
    pub fn get_ready_chunks(&self, job_id: u64, completed_chunks: &[u64]) -> Vec<JobChunk> {
        if let Some(chunks) = self.chunks_by_job.get(&job_id) {
            chunks.iter()
                .filter(|chunk| {
                    // A chunk is ready if all its dependencies are satisfied
                    chunk.dependencies.iter().all(|dep_id| completed_chunks.contains(dep_id))
                })
                .cloned()
                .collect()
        } else {
            vec![]
        }
    }
    
    pub fn record_result(&mut self, result: JobResult) -> Option<JobResult> {
        let job_id = result.parent_job_id;
        
        // Store the result
        self.results_by_job
            .entry(job_id)
            .or_insert_with(Vec::new)
            .push(result);
        
        // Check if job is complete
        let received_chunks: Vec<u64> = self.results_by_job
            .get(&job_id)
            .map(|results| results.iter().map(|r| r.chunk_id).collect())
            .unwrap_or_default();
        
        if self.result_aggregator.is_job_complete(job_id, &received_chunks) {
            // Aggregate and return final result
            if let Some(results) = self.results_by_job.get(&job_id) {
                let final_result = self.result_aggregator.aggregate_results(results.clone());
                return Some(final_result);
            }
        }
        
        None // Job not complete yet
    }
    
    pub fn cancel_job(&mut self, job_id: u64) {
        self.chunks_by_job.remove(&job_id);
        self.results_by_job.remove(&job_id);
    }
    
    fn get_job_chunks(&self, job_id: u64) -> Option<&Vec<JobChunk>> {
        self.job_chunks.get(&job_id)
    }
    
    fn get_job_results(&self, job_id: u64) -> Option<&Vec<JobResult>> {
        self.job_results.get(&job_id)
    }
    
    fn get_or_create_job_results(&mut self, job_id: u64) -> &mut Vec<JobResult> {
        self.job_results
            .entry(job_id)
            .or_default()
    }
    
    fn get_job_results_mut(&mut self, job_id: u64) -> &mut Vec<JobResult> {
        self.results_by_job
            .entry(job_id)
            .or_default()
    }
}

// Utility functions

pub fn estimate_chunk_size(data_size: usize, available_nodes: usize) -> usize {
    // Target slightly more chunks than nodes for better load balancing
    let target_chunks = available_nodes.saturating_mul(2);
    
    // Calculate chunk size
    let chunk_size = data_size.div_ceil(target_chunks);
    
    // Ensure chunk size is at least 100 records for efficiency
    std::cmp::max(chunk_size, 100)
}
