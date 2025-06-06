//! # Rust Action Dispatch Agent
//!
//! This crate implements an agent for a distributed action dispatch system. The agent connects to a central command server,
//! registers itself, listens for incoming job dispatch requests, and executes jobs as instructed.
//!
//! ## Features
//! - Connects and registers with a central command server.
//! - Listens for incoming TCP connections for job dispatch requests.
//! - Handles job execution and communication with the central server.
//! - Automatic reconnection logic for central command server failures.
//!
//! ## Environment Variables
//! - `AGENT_PORT`: The port on which the agent listens for incoming connections (default: 8081).
//! - `AGENT_NAME`: The name of the agent (default: "default_agent").
//!
//! ## Main Components
//! - [`ConnectionManager`]: Manages connections to the central command server and handles incoming job requests.
//! - [`CentralCommandWriter`]: Handles sending messages to the central command server with automatic reconnection.
//! - [`JobDispatcher`]: Responsible for executing dispatched jobs (see `job_dispatch` module).
//!
//! ## Protocol
//! - Messages are serialized and sent over TCP.
//! - Each message sent to the central command server expects an "OK" reply.
//!
//! ## Logging
//! - Uses the `tracing` crate for structured logging at various levels (info, debug, error).
//!
//! ## Example Usage
//! ```sh
//! AGENT_PORT=9000 AGENT_NAME=my_agent cargo run
//! ```
//!
//! ## Error Handling
//! - Connection attempts to the central command server are retried up to 60 times with a 5-second delay between attempts.
//! - Serialization and I/O errors are logged and handled gracefully.
//!
//! ## Extensibility
//! - The agent is designed to be extended with additional message types and job handling logic.
//!
//! ## Dependencies
//! - `tokio` for async networking
//! - `tracing` for logging
//! - `hostname` for retrieving the system hostname
//! - `core_logic::communications` for message definitions
mod job_dispatch;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use std::io;
use std::sync::Arc;
use std::{env, sync::OnceLock};

use core_logic::communications::{Message, RegisterAgent};

pub const SERVER_ADDRESS: &str = "127.0.0.1:8080";
pub const VERSION: &str = "0.1.0";

static AGENT_PORT: OnceLock<u16> = OnceLock::new();
static AGENT_NAME: OnceLock<String> = OnceLock::new();

const CHUNKS_SIZE: usize = 8192; // Size for writing messages in chunks

fn get_agent_port() -> u16 {
    *AGENT_PORT.get_or_init(|| {
        env::var("AGENT_PORT")
            .unwrap_or("8081".to_string())
            .parse()
            .expect("Invalid AGENT_PORT")
    })
}

pub fn get_agent_name() -> String {
    AGENT_NAME
        .get_or_init(|| env::var("AGENT_NAME").unwrap_or_else(|_| "default_agent".to_string()))
        .to_string()
}

fn display_agent_info() {
    info!("-------------------------------------------------");
    info!("\tRust Action Dispatch Agent");
    info!("-------------------------------------------------");
    info!(
        "\tAgent Name: {} Port: {}",
        get_agent_name(),
        get_agent_port()
    );
    info!("-------------------------------------------------");
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set the minimum level to display
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    display_agent_info();

    let mut connection_manager = ConnectionManager::try_new()
        .await
        .expect("Failed to create connection manager");

    connection_manager.register().await;
    connection_manager.listen().await?;

    Ok(())
}

/// Manages the application's connections, including the central command writer and job dispatcher.
///
/// # Fields
/// - `central_command_writer`: Shared, thread-safe writer for sending commands to the central system.
/// - `job_dispatcher`: Responsible for dispatching jobs to appropriate handlers.
pub struct ConnectionManager {
    central_command_writer: Arc<Mutex<CentralCommandWriter>>,
    job_dispatcher: job_dispatch::JobDispatcher,
}

pub struct CentralCommandWriter {
    stream: TcpStream,
}

impl CentralCommandWriter {
    pub async fn try_new() -> Result<Self, io::Error> {
        let stream = Self::connect_to_central_command().await?;

        Ok(Self { stream })
    }

    pub async fn connect_to_central_command() -> io::Result<TcpStream> {
        const MAX_ATTEMPTS: usize = 60;
        const RETRY_DELAY: u64 = 5;

        let mut attempts = 0;
        loop {
            info!("Attempting to connect to central command...");
            match TcpStream::connect(SERVER_ADDRESS).await {
                Ok(stream) => {
                    info!("Reconnected to central command.");
                    return Ok(stream);
                }
                Err(e) => {
                    info!("Failed to connect to central command: {}", e);
                    attempts += 1;
                    if attempts >= MAX_ATTEMPTS {
                        error!(
                            "Failed to reconnect to central command after {} attempts: {}",
                            e, attempts
                        );
                        return Err(e);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY)).await;
                }
            }
        }
    }

    pub async fn reconnect_to_central_command(&mut self) -> io::Result<()> {
        self.stream = Self::connect_to_central_command().await?;
        Ok(())
    }

    pub async fn write(&mut self, message: Message) {
        let serialized: Vec<u8> = match message.clone().try_into() {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to serialize message: {}", e);
                return;
            }
        };

        // Write until we get exactly "OK" from the central command
        loop {
            // Write the serialized message in chunks to avoid large buffer issues
            let mut offset = 0;
            while offset < serialized.len() {
                let end = std::cmp::min(offset + CHUNKS_SIZE, serialized.len());
                if let Err(e) = self.stream.write_all(&serialized[offset..end]).await {
                    error!("Error writing to central command: {}", e);
                    if let Err(e) = self.reconnect_to_central_command().await {
                        error!("Failed to reconnect to central command: {}", e);
                    }
                    break;
                }
                offset = end;
            }
            let mut reply = [0; 2];
            if let Err(e) = self.stream.read_exact(&mut reply).await {
                error!("Error reading reply from central command: {}", e);
                if let Err(e) = self.reconnect_to_central_command().await {
                    error!("Failed to reconnect to central command: {}", e);
                }
                continue;
            }
            if &reply == b"OK" {
                break;
            } else {
                error!("Unexpected reply from central command: {:?}", reply);
                // Optionally, you can continue to wait or break here depending on your protocol
                // For now, break to avoid infinite loop on unexpected reply
                break;
            }
        }

        debug!("Sent message to central command: {:?}", message);
    }
}

impl ConnectionManager {
    pub async fn try_new() -> io::Result<Self> {
        let central_command_writer = Arc::new(Mutex::new(CentralCommandWriter::try_new().await?));

        Ok(Self {
            central_command_writer: central_command_writer.clone(),
            job_dispatcher: job_dispatch::JobDispatcher::new(central_command_writer),
        })
    }

    async fn register(&mut self) {
        let registered_agent = RegisterAgent {
            name: get_agent_name(),
            hostname: hostname::get()
                .expect("Unable to get hostname!")
                .to_string_lossy()
                .to_string(),
            port: get_agent_port(),
        };
        let message = Message::RegisterAgent(registered_agent);
        self.central_command_writer
            .lock()
            .await
            .write(message)
            .await;
    }

    async fn ping_central_command(&mut self) {
        let message = Message::Ping;
        self.central_command_writer
            .lock()
            .await
            .write(message)
            .await;
    }

    async fn handle_message(
        &mut self,
        message: Message,
        peer_addr: std::net::SocketAddr,
    ) -> io::Result<()> {
        match message {
            Message::Ping => {
                debug!("Ping from {}", peer_addr);
                self.ping_central_command().await;
            }
            Message::DispatchJob(job) => {
                // Handle job dispatching logic here
                info!("Running job {} from {}", job.job_name, peer_addr);
                self.job_dispatcher.spawn(job).await;
            }
            _ => (),
        }
        Ok(())
    }

    pub async fn listen(&mut self) -> io::Result<()> {
        let listener = std::net::TcpListener::bind(format!("[::]:{}", get_agent_port()))?;
        listener.set_nonblocking(true)?;
        let listener = TcpListener::from_std(listener)?;

        loop {
            info!("Listening on: {}", listener.local_addr()?);
            let (mut stream, peer_addr) = listener.accept().await?;
            info!("New connection from: {}", peer_addr);

            // Spawn a new task to handle the connection
            let mut buffer = [0; 65536];

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
                                        error!("Failed to parse message: {}", e);
                                        continue;
                                    }
                                };
                                debug!("Received: {:?} from {}", message, peer_addr.ip());

                                self.handle_message(message, peer_addr).await?;

                                // Echo the data back to the client (example of keeping the connection active)
                                if let Err(e) = stream.write_all(b"OK").await {
                                    error!("Error writing to {}: {}", peer_addr, e);
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Error reading from {}: {}", peer_addr, e);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}
