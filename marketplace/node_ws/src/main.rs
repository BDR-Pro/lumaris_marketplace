//# node_ws/src/main.rs
mod ws_handler;
mod auth;
mod vm_manager;

use ws_handler::start_ws_server;

#[tokio::main]
async fn main() {
    println!("Starting WebSocket Server on ws://127.0.0.1:9001...");
    start_ws_server().await;
}