mod agent;

use core_logic::communications::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use uuid::Uuid;

use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use agent::{AgentManager, DB_AGENTS};

const SERVER_ADDRESS: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up tracing subscriber for logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set the minimum level to display
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    spawn(async move {
        let mut connection_manager = ConnectionManager::try_new()
            .await
            .expect("Failed to create connection manager");
        connection_manager
            .listen()
            .await
            .expect("Failed to listen for connections");
    });

    // Spawn a task to connect to the server and send data
    spawn(async move {
        AgentManager::default().start().await;
    });

    info!("Central Command started and listening for connections...");

    // Keep the main task alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down.");

    Ok(())
}

pub struct ConnectionManager {
    listener: TcpListener,
}

impl ConnectionManager {
    pub async fn try_new() -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind(SERVER_ADDRESS)
            .await
            .expect("Failed to bind to address");

        Ok(ConnectionManager { listener })
    }

    pub async fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        let (mut stream, peer_addr) = self.listener.accept().await?;
        info!("Accepted connection from: {}", peer_addr);

        // Spawn a new task to handle the connection
        spawn(async move {
            let mut buffer = [0; 1024];

            loop {
                tokio::select! {
                    result = stream.read(&mut buffer) => {
                        match result {
                            Ok(0) => {
                                info!("Connection with {} closed by peer.", peer_addr);
                                break; // Connection closed by the client
                            }
                            Ok(n) => {
                                let received = buffer[..n].to_vec();
                                let message: Message = received.into();

                                match message {
                                    Message::Ping => {
                                        info!("Received Ping from {}", peer_addr);
                                    }
                                    Message::RegisterAgent(agent) => {
                                        info!("Received RegisterAgent from {}", agent);
                                        let agent_addr: SocketAddr = agent.parse().unwrap();
                                        DB_AGENTS.write().await.push(agent_addr);
                                    }
                                }

                                // // Echo the data back to the client (example of keeping the connection active)
                                // if let Err(e) = stream.write_all(received).await {
                                //     error!("Error writing to {}: {}", peer_addr, e);
                                //     break;
                                // }
                            }
                            Err(e) => {
                                error!("Error reading from {}: {}", peer_addr, e);
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
