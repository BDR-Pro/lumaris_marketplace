# Lumaris Distributed Engine

This component provides the core distributed computing engine for Lumaris Marketplace. It handles job splitting, distribution, and result aggregation.

## Features

- Job splitting for parallel execution
- Dependency tracking between job chunks
- Result aggregation from distributed execution
- Execution statistics collection

## Getting Started

### Prerequisites

- Rust 1.56 or later
- Cargo package manager

### Building

```bash
cargo build --release
```

### Using as a Library

The distributed engine is designed to be used as a library by other components of the Lumaris Marketplace. Import it in your Cargo.toml:

```toml
[dependencies]
distributed_engine = { path = "../distributed_engine" }
```

## Core Components

### JobPayload

Represents the payload for a job, including command, arguments, input data, and environment variables.

### JobChunk

Represents a chunk of a job that can be executed independently, with dependencies on other chunks.

### JobResult

Represents the result of executing a job chunk, including output, errors, and execution statistics.

### JobSplitter

Trait for splitting jobs into chunks that can be executed in parallel.

### ResultAggregator

Trait for combining results from multiple chunks into a final job result.

### DistributedJobManager

Manages the lifecycle of distributed jobs, including preparation, execution, and result collection.

## Example Usage

```rust
use distributed_engine::{DistributedJobManager, JobPayload};
use std::collections::HashMap;

// Create a job manager
let mut job_manager = DistributedJobManager::default();

// Create a job payload
let payload = JobPayload {
    command: "process_data".to_string(),
    args: vec!["--input".to_string(), "file.csv".to_string()],
    input_data: Some("data,to,process\n1,2,3\n4,5,6".to_string()),
    env_vars: HashMap::new(),
};

// Prepare the job for execution
let job_id = 12345;
let chunks = job_manager.prepare_job(job_id, payload);

// Chunks can now be distributed to available nodes for execution
```

## Error Handling

The engine uses a custom error type system for consistent error handling across components.

