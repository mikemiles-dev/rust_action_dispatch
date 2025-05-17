use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info};

use std::io;
use std::{env, sync::OnceLock};

use core_logic::communications::{Message, RegisterAgent};

const SERVER_ADDRESS: &str = "127.0.0.1:8080";

static AGENT_PORT: OnceLock<u16> = OnceLock::new();
static AGENT_NAME: OnceLock<String> = OnceLock::new();

fn get_agent_port() -> u16 {
    *AGENT_PORT.get_or_init(|| {
        env::var("AGENT_PORT")
            .unwrap_or("8081".to_string())
            .parse()
            .expect("Invalid AGENT_PORT")
    })
}

fn get_agent_name() -> String {
    AGENT_NAME
        .get_or_init(|| env::var("AGENT_NAME").unwrap_or_else(|_| "default_agent".to_string()))
        .to_string()
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set the minimum level to display
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    let mut connection_manager = ConnectionManager::try_new()
        .await
        .expect("Failed to create connection manager");

    connection_manager.register().await;
    connection_manager.listen().await?;

    Ok(())
}

pub struct ConnectionManager {
    central_command_writer: CentralCommandWriter,
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

        if let Err(e) = self.stream.write_all(&serialized).await {
            error!("Error writing to central command: {}", e);
            if let Err(e) = self.reconnect_to_central_command().await {
                error!("Failed to reconnect to central command: {}", e);
            }
        }
        debug!("Sent message to central command: {:?}", message);
    }
}

impl ConnectionManager {
    pub async fn try_new() -> io::Result<Self> {
        Ok(Self {
            central_command_writer: CentralCommandWriter::try_new().await?,
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
        self.central_command_writer.write(message).await;
    }

    async fn ping_central_command(&mut self) {
        let message = Message::Ping;
        self.central_command_writer.write(message).await;
    }

    async fn report_job_complete(&mut self) {
        let message = Message::JobComplete;
        self.central_command_writer.write(message).await;
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
            Message::DispatchJob(_) => {
                // Handle job dispatching logic here
                info!("Running job from {}", peer_addr);
                self.report_job_complete().await;
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
                                        error!("Failed to parse message: {}", e);
                                        continue;
                                    }
                                };
                                debug!("Received: {:?} from {}", message, peer_addr.ip());

                                self.handle_message(message, peer_addr).await?;

                                // // Echo the data back to the client (example of keeping the connection active)
                                // if let Err(e) = stream.write_all(&[]).await {
                                //     error!("Error writing to {}: {}", peer_addr, e);
                                //     break;
                                // }

                                self.ping_central_command().await;
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
