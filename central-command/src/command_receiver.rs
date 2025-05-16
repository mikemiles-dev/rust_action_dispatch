use core_logic::{communications::Message, datastore};
use mongodb::Client;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::{error, info};

use std::sync::Arc;
use std::error::Error;

use core_logic::datastore::{Datastore,DataStoreTypes, agent::AgentV1};

const SERVER_ADDRESS: &str = "0.0.0.0:8080";

pub struct CommandReceiver {
    datastore_client: Arc<Datastore>,
    listener: TcpListener,
}

impl CommandReceiver {
    pub async fn try_new(datastore_client: Arc<Datastore>) -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind(SERVER_ADDRESS)
            .await
            .expect("Failed to bind to address");

        Ok(CommandReceiver {
            datastore_client,
            listener,
        })
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
                            Message::RegisterAgent(register_agent) => {
                                // info!(
                                //     "Received RegisterAgent from {}: {:?}",
                                //     peer_addr, register_agent
                                // );
                                // let agent: AgentV1 = register_agent.into();
                                // datastore_sender
                                //     .clone()
                                //     .send(DataStoreTypes::Agent(agent))
                                //     .await
                                //     .unwrap();
                            }
                        }
                    }
                }
            });
        }

        Ok(())
    }
}
