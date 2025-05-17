# Lumaris Matchmaking Engine Design

## Overview

The matchmaking engine will connect job submitters (buyers) with computing resource providers (sellers) based on availability, requirements, and optimal resource allocation. This document outlines the architecture and implementation plan for this core component.

## Architecture

```txt
                                 +-------------------+
                                 |                   |
                                 |  Admin API        |
                                 |  (Job Submission) |
                                 |                   |
                                 +--------+----------+
                                          |
                                          v
+------------------+            +--------------------+            +------------------+
|                  |            |                    |            |                  |
|  Seller Nodes    +<---------->+  Matchmaking       +<---------->+  Job Database   |
|  (WebSocket)     |   Stats    |  Engine            |   Query    |  (SQLite)       |
|                  |            |  (Rust)            |            |                  |
+------------------+            +--------------------+            +------------------+
                                          |
                                          v
                                +--------------------+
                                |                    |
                                |  Job Dispatcher    |
                                |  (Rust)            |
                                |                    |
                                +--------------------+
```

## Key Components

### 1. Node Availability Tracker

- Real-time monitoring of seller node status, capacity, and performance metrics
- Prioritization algorithm that considers:
  - Node reliability history
  - Current load
  - Geographic location (future feature)
  - Specialized hardware capabilities

### 2. Job Requirements Parser

- Interprets job specifications from buyers
- Extracts required:
  - CPU cores/threads
  - Memory requirements
  - Expected execution time
  - Priority level

### 3. Matching Algorithm

- Weighted scoring system to find optimal node matches
- Support for job splitting across multiple nodes
- Fallback strategies when ideal matches aren't available

### 4. Queue Management

- Priority-based job queue
- Fair scheduling to prevent resource starvation
- Support for pre-emption of lower priority jobs

## Implementation Plan

### Phase 1: Basic Matchmaking

1. Extend the WebSocket server to track node availability
2. Create a simple job-to-node assignment algorithm
3. Implement basic job dispatching

### Phase 2: Intelligent Matching

1. Add scoring and weighting to the matching algorithm
2. Implement job history tracking
3. Create node performance profiles

### Phase 3: Advanced Features

1. Implement distributed job execution across multiple nodes
2. Add fault tolerance and job migration
3. Develop predictive node availability modeling

## Database Schema Updates

```sql
-- New tables for matchmaking

CREATE TABLE job_requirements (
    id INTEGER PRIMARY KEY,
    job_id INTEGER,
    cpu_cores INTEGER,
    memory_mb INTEGER,
    expected_duration_sec INTEGER,
    priority INTEGER DEFAULT 1,
    FOREIGN KEY (job_id) REFERENCES jobs(id)
);

CREATE TABLE node_capabilities (
    id INTEGER PRIMARY KEY,
    node_id INTEGER,
    cpu_cores INTEGER,
    memory_mb INTEGER,
    reliability_score FLOAT DEFAULT 1.0,
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);

CREATE TABLE job_assignments (
    id INTEGER PRIMARY KEY,
    job_id INTEGER,
    node_id INTEGER,
    assigned_at TIMESTAMP,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    status TEXT,
    FOREIGN KEY (job_id) REFERENCES jobs(id),
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);
```

## WebSocket Protocol Extensions

The existing WebSocket protocol will be extended to include:

```json
// Node availability update
{
  "type": "availability_update",
  "node_id": "hostname123",
  "cpu_available": 3.5,  // Cores available
  "mem_available": 4096, // MB available
  "status": "ready"
}

// Job dispatch message
{
  "type": "job_dispatch",
  "job_id": 123,
  "requirements": {
    "cpu_cores": 2,
    "memory_mb": 1024,
    "expected_duration_sec": 3600
  },
  "payload": {
    "command": "execute_task.sh",
    "args": ["--input", "data.csv"]
  }
}

// Job status update
{
  "type": "job_status",
  "job_id": 123,
  "status": "running", // or "completed", "failed"
  "progress": 0.75,    // Optional: job completion percentage
  "resources_used": {
    "cpu_time_sec": 450,
    "peak_memory_mb": 820
  }
}
```

## Next Steps

1. Create the matchmaking engine skeleton
2. Extend the WebSocket handler to track node availability
3. Implement the basic matching algorithm
4. Update the database schema
5. Add job dispatch functionality
