// File: marketplace/distributed_engine/src/tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_job_splitting() {
        // Create a job splitter
        let splitter = DefaultJobSplitter::new(2); // Small chunk size for testing
        
        // Create a test job payload with line-based data
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());
        
        let payload = JobPayload {
            command: "test_command".to_string(),
            args: vec!["--test".to_string()],
            input_data: Some("line1\nline2\nline3\nline4\nline5".to_string()),
            env_vars,
        };
        
        // Split the job
        let chunks = splitter.split_job(123, payload);
        
        // We should have 3 chunks (2 data chunks + 1 aggregator)
        assert_eq!(chunks.len(), 3);
        
        // Check first chunk
        assert_eq!(chunks[0].chunk_id, 1);
        assert_eq!(chunks[0].parent_job_id, 123);
        assert_eq!(chunks[0].payload.command, "test_command");
        assert!(chunks[0].payload.input_data.as_ref().unwrap().contains("line1"));
        assert!(chunks[0].payload.input_data.as_ref().unwrap().contains("line2"));
        assert!(!chunks[0].payload.input_data.as_ref().unwrap().contains("line3"));
        
        // Check second chunk
        assert_eq!(chunks[1].chunk_id, 2);
        assert_eq!(chunks[1].parent_job_id, 123);
        assert!(chunks[1].payload.input_data.as_ref().unwrap().contains("line3"));
        assert!(chunks[1].payload.input_data.as_ref().unwrap().contains("line4"));
        
        // Check aggregator chunk
        assert_eq!(chunks[2].chunk_id, 3);
        assert_eq!(chunks[2].parent_job_id, 123);
        assert_eq!(chunks[2].payload.command, "test_command_aggregate");
        assert!(chunks[2].dependencies.contains(&1));
        assert!(chunks[2].dependencies.contains(&2));
    }

    #[test]
    fn test_result_aggregation() {
        // Create a result aggregator
        let aggregator = DefaultResultAggregator;
        
        // Create test results
        let result1 = JobResult {
            chunk_id: 1,
            parent_job_id: 123,
            success: true,
            output: Some("Result 1".to_string()),
            error: None,
            execution_stats: ExecutionStats {
                start_time: 1000,
                end_time: 1100,
                cpu_usage_percent: 50.0,
                memory_usage_mb: 100,
                error_count: 0,
            },
        };
        
        let result2 = JobResult {
            chunk_id: 2,
            parent_job_id: 123,
            success: true,
            output: Some("Result 2".to_string()),
            error: None,
            execution_stats: ExecutionStats {
                start_time: 1050,
                end_time: 1150,
                cpu_usage_percent: 60.0,
                memory_usage_mb: 120,
                error_count: 0,
            },
        };
        
        // Aggregate results
        let aggregated = aggregator.aggregate_results(vec![result1, result2]);
        
        // Check aggregated result
        assert_eq!(aggregated.parent_job_id, 123);
        assert_eq!(aggregated.success, true);
        assert!(aggregated.output.as_ref().unwrap().contains("Result 1"));
        assert!(aggregated.output.as_ref().unwrap().contains("Result 2"));
        assert_eq!(aggregated.error, None);
        
        // Check stats
        assert_eq!(aggregated.execution_stats.start_time, 1000);
        assert_eq!(aggregated.execution_stats.end_time, 1150);
        assert_eq!(aggregated.execution_stats.cpu_usage_percent, 55.0);
        assert_eq!(aggregated.execution_stats.memory_usage_mb, 120);
        assert_eq!(aggregated.execution_stats.error_count, 0);
    }
    
    #[test]
    fn test_job_manager() {
        // Create a job manager
        let mut manager = DistributedJobManager::default();
        
        // Create a test job payload
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());
        
        let payload = JobPayload {
            command: "test_command".to_string(),
            args: vec!["--test".to_string()],
            input_data: Some("line1\nline2\nline3".to_string()),
            env_vars,
        };
        
        // Prepare the job
        let job_id = 456;
        let chunks = manager.prepare_job(job_id, payload);
        
        // We should have chunks
        assert!(!chunks.is_empty());
        
        // Get ready chunks (all should be ready initially)
        let ready_chunks = manager.get_ready_chunks(job_id, &[]);
        assert!(!ready_chunks.is_empty());
        
        // Record a result for the first chunk
        let result = JobResult {
            chunk_id: chunks[0].chunk_id,
            parent_job_id: job_id,
            success: true,
            output: Some("Test output".to_string()),
            error: None,
            execution_stats: ExecutionStats {
                start_time: 1000,
                end_time: 1100,
                cpu_usage_percent: 50.0,
                memory_usage_mb: 100,
                error_count: 0,
            },
        };
        
        let final_result = manager.record_result(result);
        
        // For a single chunk job, we should get a final result
        assert!(final_result.is_some());
        assert_eq!(final_result.unwrap().parent_job_id, job_id);
    }
}

