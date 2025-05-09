use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use tracing::info;
use tracing_subscriber::prelude::*;

use core_logic::communications::{Communication, Direction, Message};

use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set the minimum level to display
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    // 1. Bind the listener to an address
    let listener = TcpListener::bind("127.0.0.1:8081").await?;
    info!("Listening on: {}", listener.local_addr()?);

    // 2. Accept incoming connections in a loop
    loop {
        // 3. Wait for a new connection
        let (mut socket, addr) = listener.accept().await?;
        info!("New client connected: {}", addr);

        // 4. Handle the new connection (e.g., read and write data)
        tokio::spawn(async move {
            let mut buf = vec![0; 65536]; // 64KB buffer
            while let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    info!("Client disconnected: {}", addr);
                    break;
                }
                // Process the received data
                let data = String::from_utf8_lossy(&buf[..n]);
                info!("Received from {}: {}", addr, data);

                // Echo the data back to the client
                socket.write_all(data.as_bytes()).await.unwrap();
            }
            info!("Finished processing connection: {}", addr);
        });
    }
}
