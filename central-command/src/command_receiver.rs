/// The `CommandReceiver` struct and its associated methods handle incoming TCP connections
/// and process messages for agent registration and job completion in a distributed system.
///
/// # Overview
/// - Listens for incoming TCP connections from agents.
/// - Processes messages such as agent registration, job completion, and pings.
/// - Interacts with a MongoDB datastore to register agents and update job statuses.
///
/// # Main Responsibilities
/// - Accept new agent connections and spawn tasks to handle each connection.
/// - Register agents in the database upon receiving a `RegisterAgent` message.
/// - Mark jobs as complete for agents and update job status when all agents have completed.
/// - Respond to agents with acknowledgments (e.g., "OK") after processing messages.
///
/// # Key Methods
/// - `new`: Creates a new `CommandReceiver` bound to a server address.
/// - `listen`: Accepts incoming TCP connections and processes messages from each agent.
/// - `process_messages`: Reads and handles messages from a TCP stream, dispatching logic based on message type.
/// - `register_agent`: Inserts a new agent into the database.
/// - `mark_agent_job_complete`: Marks an agent as having completed a job and checks if the job is fully complete.
/// - `check_job_if_all_agents_complete`: Checks if all required agents have completed a job and updates job status.
///
/// # Errors
/// Methods return `Result` types and log errors using the `tracing` crate. Errors may occur during database operations,
/// TCP communication, or message deserialization.
///
/// # Usage
/// Typically, create a `CommandReceiver` with a shared `Datastore` client and call `listen()` to start accepting connections.
///
/// # Example
/// ```rust
/// let datastore = Arc::new(Datastore::new(...));
/// let mut receiver = CommandReceiver::new(datastore).await;
/// receiver.listen().await?;
/// ```
use bson::{Array, Document, doc};
use core_logic::{
    datastore::runs::RunsV1,
    messages::{JobComplete, Message, RegisterAgent},
};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::spawn;
use tracing::{debug, error, info, warn};

use std::error::Error;
use std::sync::Arc;

use crate::SERVER_ADDRESS;
use core_logic::datastore::{Datastore, agents::AgentV1, jobs::Status};
use tokio::io::AsyncWriteExt;

const CHUNKS_SIZE: usize = 4096; // Size of each message chunk

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
        let db = datastore_client.get_database();
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

    pub async fn check_job_completion(
        datastore_client: Arc<Datastore>,
        job_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let db = datastore_client.get_database();
        let jobs_collection = db.collection::<Document>("jobs");

        let filter = doc! { "name": job_name };
        let job_doc = jobs_collection.find_one(filter.clone()).await?;

        let Some(job_doc) = job_doc else {
            debug!("Job {} not found.", job_name);
            return Ok(());
        };

        let agents_required = match job_doc.get_array("agents_required") {
            Ok(arr) => arr,
            Err(_) => {
                debug!("Job {} missing 'agents_required' field.", job_name);
                return Ok(());
            }
        };

        let agents_complete = match job_doc.get_array("agents_complete") {
            Ok(arr) => arr,
            Err(_) => {
                debug!("Job {} missing 'agents_complete' field.", job_name);
                return Ok(());
            }
        };

        if agents_required.len() == agents_complete.len() && !agents_required.is_empty() {
            info!("Completed job {}", job_name);

            let update = doc! {
                "$set": {
                    "status": Status::Completed,
                    "agents_running": Array::new(),
                    "agents_complete": Array::new(),
                }
            };
            jobs_collection.update_one(filter, update).await?;
        } else {
            debug!("Job {} is not yet complete.", job_name);
        }

        Ok(())
    }

    /// Adds an agent to the `agents_complete` list of a job in the database.
    /// This function updates the `jobs` collection in the MongoDB database,
    /// adding the agent's name to the `agents_complete` array for the specified job.
    pub async fn complete_agent_run(
        datastore_client: Arc<Datastore>,
        job_complete: JobComplete,
        peer_addr: std::net::SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        let db = datastore_client.get_database();
        let jobs_collection = db.collection::<Document>("jobs");

        let agent_name = job_complete.agent_name.clone();
        let job_name = job_complete.job_name.clone();

        // Find job name
        let filter = doc! { "name": &job_name };
        // Update the job
        let update = doc! {
            "$addToSet": { "agents_complete": &agent_name },
        };

        info!("{agent_name} on {} Completed {job_name}", peer_addr);

        match jobs_collection.update_one(filter, update).await {
            Ok(result) => {
                if result.modified_count > 0 {
                    info!("Agent {} finished to job {}", agent_name, job_name);
                } else {
                    warn!("No job found with name {}", job_name);
                }
            }
            Err(e) => {
                error!("Failed to update job {}: {}", job_name, e);
            }
        }

        // Mark the agent as having completed the job
        let run: RunsV1 = job_complete.into();
        run.insert_entry(&db).await?;

        drop(db);

        Self::check_job_completion(datastore_client.clone(), &job_name).await
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
        loop {
            let msg_len = match Self::read_message_length(stream, peer_addr).await? {
                Some(len) => len,
                None => break, // Connection closed
            };

            let received_data = Self::read_message_body(stream, msg_len, peer_addr).await?;
            let message: Message = received_data.try_into()?;

            // Send an OK reply to the agent after job complete
            if let Err(e) = stream.write_all(b"OK").await {
                error!("Failed to send OK reply to {}: {}", peer_addr, e);
            }

            Self::handle_message(message, datastore_client.clone(), peer_addr).await?;
        }
        Ok(())
    }

    async fn read_message_length(
        stream: &mut tokio::net::TcpStream,
        peer_addr: std::net::SocketAddr,
    ) -> Result<Option<usize>, Box<dyn Error>> {
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf).await {
            Ok(_) => {
                let msg_len = u32::from_be_bytes(len_buf) as usize;
                if msg_len == 0 {
                    warn!("Received zero-length message from {}", peer_addr);
                    Ok(None)
                } else {
                    Ok(Some(msg_len))
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    info!("Connection with {} closed by peer.", peer_addr);
                    Ok(None)
                } else {
                    error!("Failed to read message length from {}: {}", peer_addr, e);
                    Ok(None)
                }
            }
        }
    }

    async fn read_message_body(
        stream: &mut tokio::net::TcpStream,
        msg_len: usize,
        peer_addr: std::net::SocketAddr,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut received_data = Vec::with_capacity(msg_len);
        while received_data.len() < msg_len {
            let to_read = std::cmp::min(CHUNKS_SIZE, msg_len - received_data.len());
            let mut buffer = vec![0u8; to_read];
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                info!(
                    "Connection with {} closed while reading message.",
                    peer_addr
                );
                return Err("Connection closed while reading message".into());
            }
            received_data.extend_from_slice(&buffer[..n]);
        }
        Ok(received_data)
    }

    async fn handle_message(
        message: Message,
        datastore_client: Arc<Datastore>,
        peer_addr: std::net::SocketAddr,
    ) -> Result<(), Box<dyn Error>> {
        match message {
            Message::Ping => {
                debug!("Ping received from {}", peer_addr);
            }
            Message::RegisterAgent(register_agent) => {
                Self::register_agent(datastore_client, register_agent).await;
            }
            Message::JobComplete(job_complete) => {
                Self::complete_agent_run(datastore_client, job_complete, peer_addr).await?;
            }
            _ => (),
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
                    Self::process_messages(&mut stream, datastore_client, peer_addr).await
                {
                    error!("Error processing messages from {}: {}", peer_addr, e);
                }
            });
        }

        Ok(())
    }
}
