mod agent;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use tokio::sync::mpsc;

use std::error::Error;
use std::net::SocketAddr;

use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up tracing subscriber for logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set the minimum level to display
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on: {}", addr);

    let (tx, mut rx) = mpsc::channel::<String>(32);

    // Spawn a task to handle incoming connections
    spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            info!("Accepted connection from: {}", addr);
            let tx_clone = tx.clone();
            spawn(async move {
                if let Err(e) = handle_connection(stream, tx_clone).await {
                    error!("Error handling connection from {}: {}", addr, e);
                }
            });
        }
    });

    // Spawn a task to connect to the server and send data
    spawn(async move {
        let server_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        match TcpStream::connect(server_addr).await {
            Ok(mut stream) => {
                info!("Connected to server!");
                while let Some(message) = rx.recv().await {
                    info!("Sending: {}", message);
                    if let Err(e) = stream.write_all(message.as_bytes()).await {
                        error!("Error sending data: {}", e);
                        break;
                    }
                    if let Err(e) = stream.write_all(b"\n").await {
                        // Add newline as a message delimiter
                        error!("Error sending newline: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                error!("Error connecting to server: {}", e);
            }
        }
    });

    // Keep the main task alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down.");

    Ok(())
}

async fn handle_connection(
    mut stream: TcpStream,
    tx: mpsc::Sender<String>,
) -> Result<(), Box<dyn Error>> {
    let addr = stream.peer_addr()?;
    let mut buffer = [0; 1024];

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                info!("Connection with {} closed by peer.", addr);
                break;
            }
            Ok(n) => {
                match String::from_utf8(buffer[..n].to_vec()) {
                    Ok(message) => {
                        info!("Received from {}: {}", addr, message.trim());
                        // Optionally, broadcast the received message to other connected clients
                        if tx
                            .send(format!("{}: {}", addr, message.trim()))
                            .await
                            .is_err()
                        {
                            error!("Error sending received message to broadcast channel");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Received invalid UTF-8 from {}: {}", addr, e);
                        break;
                    }
                }
            }
            Err(e) => {
                error!("Error reading from {}: {}", addr, e);
                break;
            }
        }
    }

    Ok(())
}
