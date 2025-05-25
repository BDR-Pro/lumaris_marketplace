// File: marketplace/distributed_engine/src/lib.rs

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::ops::Div;

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
    pub result: Option<JobResult>, // Store the result of this chunk
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

// Define JobData for job processing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobData {
    pub job_id: u64,
    pub payload: JobPayload,
    pub priority: u8,
    pub max_retries: u8,
}

// Core interfaces for the distributed engine

pub trait JobSplitter: Send + Sync {
    /// Split a job into multiple chunks that can be executed in parallel
    fn split_job(&self, job_data: &JobData) -> Vec<JobChunk>;
    
    /// Calculate dependencies between job chunks
    fn calculate_dependencies(&self, chunks: &mut Vec<JobChunk>);
    
    /// Split a text job into chunks
    fn split_text_job(&self, job_id: u64, text: &str) -> Vec<JobChunk>;
}

pub trait ResultAggregator: Send + Sync {
    /// Combine results from multiple chunks into a final job result
    fn aggregate_results(&self, results: Vec<JobResult>) -> JobResult;
    
    /// Check if all required chunks have completed
    fn is_job_complete(&self, job_id: u64, received_chunks: &[u64]) -> bool;
}

pub trait JobProcessor: Send + Sync {
    /// Process a job chunk and return a result
    fn process_chunk(&self, job_id: u64, chunk: &JobChunk) -> String;
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
    fn split_job(&self, job_data: &JobData) -> Vec<JobChunk> {
        // If the payload has input data, split it by lines
        if let Some(data) = &job_data.payload.input_data {
            let lines: Vec<&str> = data.lines().collect();
            
            // Calculate number of chunks based on input size and threshold
            let num_chunks = (lines.len() + self.chunk_size_threshold - 1) / self.chunk_size_threshold;
            
            // Create chunks
            let mut chunks = Vec::with_capacity(num_chunks + 1); // +1 for aggregator
            
            for i in 0..num_chunks {
                let start = i * self.chunk_size_threshold;
                let end = std::cmp::min((i + 1) * self.chunk_size_threshold, lines.len());
                
                let chunk_data = lines[start..end].join("\n");
                
                let chunk_payload = JobPayload {
                    command: job_data.payload.command.clone(),
                    args: job_data.payload.args.clone(),
                    input_data: Some(chunk_data),
                    env_vars: job_data.payload.env_vars.clone(),
                };
                
                let chunk = JobChunk {
                    chunk_id: (i + 1) as u64,
                    parent_job_id: job_data.job_id,
                    payload: chunk_payload,
                    dependencies: Vec::new(),
                    estimated_work_units: (end - start) as u64,
                    result: None,
                };
                
                chunks.push(chunk);
            }
            
            // Create an aggregator chunk that will combine results
            let aggregator_payload = JobPayload {
                command: format!("{}_aggregate", job_data.payload.command),
                args: job_data.payload.args.clone(),
                input_data: None,  // Aggregator doesn't need input data
                env_vars: job_data.payload.env_vars.clone(),
            };
            
            let aggregator_chunk = JobChunk {
                chunk_id: (num_chunks + 1) as u64,
                parent_job_id: job_data.job_id,
                payload: aggregator_payload,
                dependencies: (1..=num_chunks as u64).collect(), // Depends on all other chunks
                estimated_work_units: 10, // Small fixed cost for aggregation
                result: None,
            };
            
            chunks.push(aggregator_chunk);
            
            chunks
        } else {
            // If no input data, just create a single chunk
            vec![JobChunk {
                chunk_id: 1,
                parent_job_id: job_data.job_id,
                payload: job_data.payload.clone(),
                dependencies: Vec::new(),
                estimated_work_units: 100, // Default work units
                result: None,
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
    
    fn split_text_job(&self, job_id: u64, text: &str) -> Vec<JobChunk> {
        // Split the text into lines
        let lines: Vec<&str> = text.lines().collect();
        
        if lines.is_empty() {
            return vec![];
        }
        
        // Calculate the number of chunks
        let num_chunks = lines.len().div_ceil(self.chunk_size_threshold);
        
        // Create chunks
        let mut chunks = Vec::with_capacity(num_chunks);
        
        for i in 0..num_chunks {
            let start = i * self.chunk_size_threshold;
            let end = std::cmp::min((i + 1) * self.chunk_size_threshold, lines.len());
            
            let chunk_data = lines[start..end].join("\n");
            
            let chunk_payload = JobPayload {
                command: "process".to_string(),
                args: vec![],
                input_data: Some(chunk_data),
                env_vars: HashMap::new(),
            };
            
            let chunk = JobChunk {
                chunk_id: (i + 1) as u64,
                parent_job_id: job_id,
                payload: chunk_payload,
                dependencies: Vec::new(),
                estimated_work_units: (end - start) as u64,
                result: None,
            };
            
            chunks.push(chunk);
        }
        
        chunks
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

// Simple default job processor
pub struct DefaultJobProcessor;

impl JobProcessor for DefaultJobProcessor {
    fn process_chunk(&self, _job_id: u64, chunk: &JobChunk) -> String {
        // In a real implementation, this would actually process the chunk
        // For now, just return a dummy result
        format!("Processed chunk {} for job {}", chunk.chunk_id, chunk.parent_job_id)
    }
}

// Job execution coordination

pub struct DistributedJobManager {
    job_splitter: Box<dyn JobSplitter>,
    result_aggregator: Box<dyn ResultAggregator>,
    job_processor: Box<dyn JobProcessor>,
    chunks_by_job: HashMap<u64, Vec<JobChunk>>,
    results_by_job: HashMap<u64, Vec<JobResult>>,
}

impl DistributedJobManager {
    pub fn new(
        job_splitter: Box<dyn JobSplitter>,
        result_aggregator: Box<dyn ResultAggregator>,
        job_processor: Box<dyn JobProcessor>
    ) -> Self {
        Self {
            job_splitter,
            result_aggregator,
            job_processor,
            chunks_by_job: HashMap::new(),
            results_by_job: HashMap::new(),
        }
    }
    
    /// Create a new DistributedJobManager with default implementations
    #[must_use]
    pub fn new_default() -> Self {
        Self::new(
            Box::new(DefaultJobSplitter::new(1000)),
            Box::new(DefaultResultAggregator),
            Box::new(DefaultJobProcessor)
        )
    }
    
    pub fn prepare_job(&mut self, job_id: u64, payload: JobPayload) -> Vec<JobChunk> {
        // Create a JobData from the payload
        let job_data = JobData {
            job_id,
            payload,
            priority: 1,
            max_retries: 3,
        };
        
        // Split the job into chunks
        let mut chunks = self.job_splitter.split_job(&job_data);
        
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
            .or_default()
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
    
    fn get_job_results(&self, job_id: u64) -> Option<&Vec<JobResult>> {
        self.results_by_job.get(&job_id)
    }

    #[allow(dead_code)]
    fn get_job_chunks(&self, job_id: u64) -> Option<&Vec<JobChunk>> {
        self.chunks_by_job.get(&job_id)
    }

    #[allow(dead_code)]
    fn get_or_create_job_results(&mut self, job_id: u64) -> &mut Vec<JobResult> {
        self.results_by_job
            .entry(job_id)
            .or_default()
    }

    #[allow(dead_code)]
    fn get_job_results_mut(&mut self, job_id: u64) -> &mut Vec<JobResult> {
        self.results_by_job
            .entry(job_id)
            .or_default()
    }
    
    fn add_job_chunk(&mut self, job_id: u64, chunk: JobChunk) {
        self.chunks_by_job
            .entry(job_id)
            .or_default()
            .push(chunk);
    }
    
    fn add_chunk_to_job(&mut self, job_id: u64, chunk: JobChunk) {
        self.chunks_by_job
            .entry(job_id)
            .or_default()
            .push(chunk);
    }
    
    fn add_job_result(&mut self, job_id: u64, result: JobResult) {
        self.results_by_job
            .entry(job_id)
            .or_default()
            .push(result);
    }
    
    fn create_job_chunks(&mut self, job_id: u64, job_data: &JobData) -> Vec<JobChunk> {
        let chunks = self.job_splitter.split_job(job_data);
        
        // Store chunks for this job
        self.chunks_by_job
            .entry(job_id)
            .or_default()
            .extend(chunks.clone());
            
        chunks
    }
    
    fn process_job(&mut self, job_id: u64, job_data: &JobData) -> Vec<JobResult> {
        // Create chunks for this job
        let chunks = self.create_job_chunks(job_id, job_data);
        
        // Process each chunk and collect results
        let mut results = Vec::new();
        
        for chunk in chunks {
            // In a real system, this would be distributed to worker nodes
            // For now, we process locally
            let result = self.process_chunk(job_id, &chunk);
            results.push(result.clone());
            
            // Store the result
            self.add_job_result(job_id, result);
        }
        
        // Return all results
        results
    }
    
    fn process_chunk(&self, job_id: u64, chunk: &JobChunk) -> JobResult {
        // In a real system, this would be sent to a worker node
        // For now, we just simulate processing
        
        // Call the processor to handle this chunk
        let result_data = self.job_processor.process_chunk(job_id, chunk);
        
        JobResult {
            chunk_id: chunk.chunk_id,
            parent_job_id: job_id,
            success: true,
            output: Some(result_data),
            error: None,
            execution_stats: ExecutionStats {
                start_time: chrono::Utc::now().timestamp() as u64,
                end_time: chrono::Utc::now().timestamp() as u64 + 1,
                cpu_usage_percent: 10.0,
                memory_usage_mb: 100,
                error_count: 0,
            },
        }
    }
    
    fn add_job_chunk_result(&mut self, job_id: u64, chunk_id: u64, result: JobResult) {
        self.chunks_by_job
            .entry(job_id)
            .or_default();
            
        // Find the chunk and update its result
        if let Some(chunks) = self.chunks_by_job.get_mut(&job_id) {
            if let Some(chunk) = chunks.iter_mut().find(|c| c.chunk_id == chunk_id) {
                chunk.result = Some(result.clone());
            }
        }
        
        // Also store in results collection
        self.add_job_result(job_id, result);
    }
}

// Utility functions

pub fn estimate_chunk_size(data_size: usize, available_nodes: usize) -> usize {
    // Target slightly more chunks than nodes for better load balancing
    let target_chunks = available_nodes.saturating_mul(2);
    
    // Calculate chunk size
    let chunk_size = (data_size + target_chunks - 1) / target_chunks;
    
    // Ensure chunk size is at least 100 records for efficiency
    std::cmp::max(chunk_size, 100)
}
