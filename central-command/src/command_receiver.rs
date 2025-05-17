use bson::Document;
use core_logic::communications::{Message, RegisterAgent};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::{error, info, warn};

use std::error::Error;
use std::sync::Arc;

use core_logic::datastore::{Datastore, agents::AgentV1};

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

    async fn register_agent(datastore_client: Arc<Datastore>, register_agent: RegisterAgent) {
        let db = datastore_client.client.database("rust-action-dispatch");
        let agents_collection = db.collection::<Document>("agents");
        let agent: AgentV1 = register_agent.into();
        let bson_agent = match bson::to_document(&agent) {
            Ok(doc) => doc,
            Err(e) => {
                error!("Failed to convert agent to BSON: {}", e);
                return;
            }
        };
        let result = agents_collection.insert_one(bson_agent, None).await;
        match result {
            Ok(_) => {
                info!("Inserted agent: {:?}", agent);
            }
            Err(e) => {
                warn!("Failed to insert agent: {}, {}", agent, e);
            }
        }
    }

    #[allow(unreachable_code)]
    pub async fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let datastore_client = self.datastore_client.clone();
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
                                Self::register_agent(datastore_client.clone(), register_agent)
                                    .await;
                            }
                        }
                    }
                }
            });
        }

        Ok(())
    }
}
