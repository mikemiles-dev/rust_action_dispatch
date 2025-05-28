use bson::{Document, doc};
use core_logic::communications::{JobComplete, Message, RegisterAgent};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::{error, info, warn};

use std::error::Error;
use std::sync::Arc;

use crate::SERVER_ADDRESS;
use core_logic::datastore::{Datastore, agents::AgentV1};

pub struct CommandReceiver {
    datastore_client: Arc<Datastore>,
    listener: TcpListener,
}

impl CommandReceiver {
    pub async fn new(datastore_client: Arc<Datastore>) -> Self {
        let listener = TcpListener::bind(SERVER_ADDRESS)
            .await
            .expect("Failed to bind to address");

        CommandReceiver {
            datastore_client,
            listener,
        }
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
        let result = agents_collection.insert_one(bson_agent).await;
        match result {
            Ok(_) => {
                info!("Inserted agent: {:?}", agent);
            }
            Err(e) => {
                warn!("Failed to insert agent: {}, {}", agent, e);
            }
        }
    }

    pub async fn add_agent_complete_to_job(
        datastore_client: Arc<Datastore>,
        job_name: &str,
        agent_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let db = datastore_client.client.database("rust-action-dispatch");
        let jobs_collection = db.collection::<Document>("jobs");

        let filter = doc! { "name": job_name };
        let update = doc! { "$addToSet": { "agents_complete": agent_name } };

        match jobs_collection.update_one(filter, update).await {
            Ok(result) => {
                if result.modified_count > 0 {
                    info!("Added agent {} to job {}", agent_name, job_name);
                } else {
                    warn!("No job found with name {}", job_name);
                }
            }
            Err(e) => {
                error!("Failed to update job {}: {}", job_name, e);
            }
        }

        Ok(())
    }

    /// Processes incoming messages from the TCP stream.
    /// /// This function reads messages from the stream, deserializes them into `Message` enum variants,
    pub async fn process_messages(
        stream: &mut tokio::net::TcpStream,
        datastore_client: Arc<Datastore>,
        peer_addr: std::net::SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        let mut buffer = [0; 65536];
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    info!("Connection with {} closed by peer.", peer_addr);
                    break; // Connection closed by the client
                }
                Ok(n) => {
                    let received = buffer[..n].to_vec();
                    let message: Message = received.try_into()?;

                    match message {
                        Message::Ping => {
                            info!("Ping received from {}", peer_addr);
                        }
                        Message::RegisterAgent(register_agent) => {
                            Self::register_agent(datastore_client.clone(), register_agent).await
                        }
                        Message::JobComplete(job_name) => {
                            let JobComplete {
                                job_name,
                                agent_name,
                            } = job_name.clone();
                            info!(
                                "Job {job_name} completed on {agent_name} from {}",
                                peer_addr
                            );
                            Self::add_agent_complete_to_job(
                                datastore_client.clone(),
                                &job_name,
                                &agent_name,
                            )
                            .await?;
                        }
                        _ => (),
                    }
                }
                Err(e) => {
                    error!("Failed to read from connection {}: {}", peer_addr, e);
                    break;
                }
            }
        }
        Ok(())
    }

    #[allow(unreachable_code)]
    pub async fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let datastore_client = self.datastore_client.clone();
            let (mut stream, peer_addr) = self.listener.accept().await?;
            info!("Accepted connection from: {}", peer_addr);
            if let Err(e) = Self::process_messages(&mut stream, datastore_client, peer_addr).await {
                error!("Error processing messages from {}: {}", peer_addr, e);
            }
            spawn(async move {});
        }

        Ok(())
    }
}
