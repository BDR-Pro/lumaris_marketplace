# Lumaris: Distributed Compute Marketplace

Lumaris is a decentralized marketplace for computing resources, enabling individuals or organizations to rent out spare CPU and memory capacity via a secure and trackable system. Buyers can submit computational tasks, and sellers (nodes) can offer their computing power in exchange for earnings.

---

## 🚀 Project Goals

### 🔧 Core Objectives

* Enable users to **buy and sell CPU and memory resources**.
* Ensure **isolation** using lightweight VMs (Firecracker).
* Offer **real-time dashboards** for sellers and **tracking APIs** for admins.
* Provide a secure and efficient **matchmaking and job dispatch system**.
* Support **distributed computing**, allowing tasks to run across multiple nodes.

---

## ✅ Features Implemented

### 🧠 Seller Node GUI (Rust + Egui)

* Built with `eframe` and `egui`.
* Displays real-time stats:

  * CPU usage
  * Memory usage
  * Node ID
  * Funds earned
* Communicates via WebSocket and REST to broadcast stats to central server.

### 📡 WebSocket Server (Rust)

* Accepts connections from seller nodes.
* Receives and logs system stats in real time.
* Simple broadcast mechanism for monitoring.

### 📊 Admin API (Python FastAPI)

* Endpoints:

  * `/nodes/` - List all registered nodes.
  * `/nodes/update` - Accepts stat updates from nodes.
  * `/jobs/` - List submitted jobs.
* SQLite backend for prototyping.
* Pydantic models for validation.
* Modular structure (`models.py`, `schemas.py`, `nodes.py`, `jobs.py`).

### 🔐 Authentication (Planned)

* Placeholder for integrating JWT-based auth for:

  * Admins
  * Nodes (future token handshake)

### 🔄 Resource Metrics (Rust, `sysinfo`, `winapi`)

* Accurate CPU usage via low-level Windows API.
* Memory usage calculated via `sysinfo`.

---

## 🛠️ Features In Progress / To Do

### 🧪 Firecracker VM Integration

* [ ] Use Firecracker to spawn minimal VMs for job execution.
* [ ] Each job is executed in an isolated microVM.
* [ ] Job lifecycle: schedule -> run -> report.

### 🛠️ Matchmaking Engine (Rust)

* [ ] WebSocket-based live availability tracking.
* [ ] Notify buyers of available nodes.
* [ ] Assign jobs fairly across the network.

### 🔗 Distributed Computing

* [ ] Split jobs across multiple nodes.
* [ ] Support for chunked job dispatch.
* [ ] Merge results from distributed execution.

### 💸 Payment and Billing Engine

* [ ] Track usage time per job.
* [ ] Charge buyers, credit sellers.
* [ ] Integrate future payment APIs (e.g., Stripe, crypto).

### 🌐 Frontend Dashboard (Future)

* [ ] Admin dashboard for monitoring jobs, revenue.
* [ ] Buyer dashboard to submit and track jobs.
* [ ] Seller node registration and management.

---

## 🧱 Architecture Summary

```text
+------------------+        +--------------------+        +-------------------------+
|   Buyer Client   | <--->  |   Admin REST API   | <--->  |       SQLite DB        |
+------------------+        +--------------------+        +-------------------------+
         |                          ^                               ^
         v                          |                               |
+------------------+               |                               |
|  WebSocket Match |  <------------+                               |
|  (Rust Server)   | <---------------------------------------------+
+------------------+     Stats / Availability Updates
         ^
         |
+------------------+
|  Seller Node GUI |
|     (Rust)       |
+------------------+
```

---

## 📦 Tech Stack

| Component          | Technology            |
| ------------------ | --------------------- |
| GUI                | Rust + Egui           |
| Admin API          | Python + FastAPI      |
| Job Dispatch / WS  | Rust + Tungstenite    |
| VM Execution       | Firecracker (planned) |
| Metrics Collection | sysinfo + winapi      |
| Database           | SQLite                |
| Package Manager    | Cargo / pip           |

---

## 📝 Getting Started

1. **Clone Repository**

   ```bash
   git clone https://github.com/bdr-pro/lumaris_marketplace.git
   ```

2. **Run Admin API**

   ```bash
   cd marketplace/admin_api
   uvicorn main:app --reload
   ```

3. **Run Node GUI**

   ```bash
   cd marketplace/rust_gui
   cargo run
   ```

4. **Start WebSocket Server**

   ```bash
   cd marketplace/node_ws
   cargo run
   ```

---

## 🤝 Contributors

* **Bader Alotaibi** – Vision, Rust GUI, architecture

---

## Matchmaking Engine

* [Match Making Design](matchmaking_design.md)

## 🔮 Future Vision

> A decentralized, pay-per-cycle computing marketplace allowing researchers, developers, and enterprises to access elastic CPU/GPU power from global contributors.

Stay tuned for:

* Docker orchestration
* GPU support
* P2P job relaying
* Zero-trust execution environments
