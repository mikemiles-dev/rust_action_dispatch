use core_logic::communications::Message;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::{error, info};

use std::error::Error;

use crate::agent::DB_AGENTS;

const SERVER_ADDRESS: &str = "127.0.0.1:8080";

pub struct CommandReceiver {
    listener: TcpListener,
}

impl CommandReceiver {
    pub async fn try_new() -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind(SERVER_ADDRESS)
            .await
            .expect("Failed to bind to address");

        Ok(CommandReceiver { listener })
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
                                let message: Message = match received.try_into() {
                                    Ok(msg) => msg,
                                    Err(e) => {
                                        error!("Failed to deserialize message: {}", e);
                                        continue;
                                    }
                                };

                                match message {
                                    Message::Ping => {
                                        info!("Received Ping from {}", peer_addr);
                                    }
                                    Message::RegisterAgent(agent_port) => {
                                        let mut agent_addr = peer_addr;
                                        agent_addr.set_port(agent_port);
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
