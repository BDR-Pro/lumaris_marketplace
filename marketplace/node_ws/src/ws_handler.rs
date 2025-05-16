use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::StreamExt;

pub async fn start_ws_server() {
    let listener = TcpListener::bind("127.0.0.1:9001").await.expect("Failed to bind");
    println!("Listening on ws://127.0.0.1:9001");

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(stream: tokio::net::TcpStream) {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    println!("New connection");

    let (_, mut read) = ws_stream.split();

    while let Some(Ok(msg)) = read.next().await {
        if let Message::Text(text) = msg {
            println!("Received stats: {}", text);
            // TODO: Save to DB or log file
        }
    }
}
