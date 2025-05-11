use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use tracing::{error, info};

use core_logic::communications::Message;

use std::io;

const SERVER_ADDRESS: &str = "127.0.0.1:8080";
const AGENT_STRING: &str = "127.0.0.1:8081";

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
        let serialized: Vec<u8> = match message.try_into() {
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
    }
}

impl ConnectionManager {
    pub async fn try_new() -> io::Result<Self> {
        Ok(Self {
            central_command_writer: CentralCommandWriter::try_new().await?,
        })
    }

    pub async fn register(&mut self) {
        let message = Message::RegisterAgent(AGENT_STRING.to_string());
        self.central_command_writer.write(message).await;
    }

    pub async fn ping_central_command(&mut self) {
        let message = Message::Ping;
        self.central_command_writer.write(message).await;
    }

    pub async fn listen(&mut self) -> io::Result<()> {
        let listener = std::net::TcpListener::bind(AGENT_STRING)?;
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
                                info!("Received: {:?} from {}", message, peer_addr.ip());

                                // Echo the data back to the client (example of keeping the connection active)
                                if let Err(e) = stream.write_all(&vec![]).await {
                                    error!("Error writing to {}: {}", peer_addr, e);
                                    break;
                                }

                                self.ping_central_command().await;
                            }
                            Err(e) => {
                                error!("Error reading from {}: {}", peer_addr, e);
                                break;
                            }
                        }
                    }
                    // You could add other asynchronous tasks here that might interact with this connection
                    // For example, a timer or a channel receiver.
                }
            }
        }
    }
}
