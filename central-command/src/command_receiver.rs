use core_logic::communications::Message;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::{error, info};

use std::error::Error;

use crate::agent::DB_AGENTS;

const SERVER_ADDRESS: &str = "0.0.0.0:8080";

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

    #[allow(unreachable_code)]
    pub async fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let (mut stream, peer_addr) = self.listener.accept().await?;
            info!("Accepted connection from: {}", peer_addr);

            // Spawn a new task to handle the connection
            spawn(async move {
                let mut buffer = [0; 1024];
                loop {
                    let n = stream.read(&mut buffer).await.unwrap();
                    if n == 0 {
                        info!("Connection with {} closed by peer.", peer_addr);
                        break; // Connection closed by the client
                    } else {
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
                    }
                }
            });
        }

        Ok(())
    }
}
