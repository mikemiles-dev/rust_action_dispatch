use bson::{Document, doc};
use core_logic::communications::{JobComplete, Message, RegisterAgent};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::{error, info, warn};

use std::error::Error;
use std::sync::Arc;

use crate::SERVER_ADDRESS;
use core_logic::datastore::{Datastore, agents::AgentV1, jobs::Status};

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

    /// Registers an agent in the database.
    /// This function takes a `RegisterAgent` message, converts it to an `AgentV1` struct,
    /// and inserts it into the `agents` collection in the MongoDB database.
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

    pub async fn check_job_if_all_agents_complete(
        datastore_client: Arc<Datastore>,
        job_name: &str,
    ) -> Result<bool, Box<dyn Error>> {
        let db = datastore_client.client.database("rust-action-dispatch");
        let jobs_collection = db.collection::<Document>("jobs");

        let filter = doc! { "name": job_name };
        let job: Option<Document> = jobs_collection.find_one(filter.clone()).await?;

        if let Some(job_doc) = job {
            if let Some(agents_required) = job_doc.get_array("agents_required").ok() {
                if let Some(agents_complete) = job_doc.get_array("agents_complete").ok() {
                    if agents_required.len() == agents_complete.len() {
                        info!("All agents have completed job {}", job_name);

                        let update = doc! {
                            "$set": {
                                "status": Status::Completed,
                                "agents_running": []
                            }
                        };
                        jobs_collection.update_one(filter, update).await?;

                        return Ok(true);
                    }
                }
            }
        }
        info!("Job {} is not yet complete.", job_name);
        Ok(false)
    }

    /// Adds an agent to the `agents_complete` list of a job in the database.
    /// This function updates the `jobs` collection in the MongoDB database,
    /// adding the agent's name to the `agents_complete` array for the specified job.
    pub async fn mark_agent_job_complete(
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

        Self::check_job_if_all_agents_complete(datastore_client, job_name).await?;

        Ok(())
    }

    /// Processes incoming messages from the TCP stream.
    /// This function reads messages from the stream, deserializes them into `Message` enum variants,
    /// and handles each message type accordingly.
    /// It handles `Ping`, `RegisterAgent`, and `JobComplete` messages.
    /// If the connection is closed by the client, it logs the event and exits the loop.
    /// If an error occurs while reading from the stream, it logs the error and exits the loop.
    /// Returns `Ok(())` if successful, or an error if something goes wrong.
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
                            Self::mark_agent_job_complete(
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

    /// Listens for incoming TCP connections and processes messages.
    /// This function accepts incoming connections, spawns a new task for each connection,
    /// and processes messages from the stream using `process_messages`.
    /// It runs indefinitely, accepting connections and processing messages until an error occurs.
    #[allow(unreachable_code)]
    pub async fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let datastore_client = self.datastore_client.clone();
            let (mut stream, peer_addr) = self.listener.accept().await?;
            spawn(async move {
                info!("Accepted connection from: {}", peer_addr);
                if let Err(e) =
                    Self::process_messages(&mut stream, datastore_client.clone(), peer_addr).await
                {
                    error!("Error processing messages from {}: {}", peer_addr, e);
                }
            });
        }

        Ok(())
    }
}
