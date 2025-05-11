use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

use tracing::{debug, error, info};

use core_logic::communications::{Communication, Direction, Message};

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

    let connection_manager =
        ConnectionManager::try_new().expect("Failed to create connection manager");

    connection_manager.register().await;

    connection_manager.listen().await?;

    Ok(())
}

pub struct ConnectionManager {
    listener: TcpListener,
}

impl ConnectionManager {
    pub fn try_new() -> io::Result<Self> {
        let listener = std::net::TcpListener::bind(AGENT_STRING)?;
        listener.set_nonblocking(true)?;
        let listener = TcpListener::from_std(listener)?;

        Ok(Self { listener })
    }

    pub async fn register(&self) {
        match TcpStream::connect(SERVER_ADDRESS).await {
            Ok(mut stream) => {
                info!("Connected to server at {}", SERVER_ADDRESS);
                let message = Message::RegisterAgent(AGENT_STRING.to_string());
                let serialized: Vec<u8> = message.into();
                if let Err(e) = stream.write_all(&serialized).await {
                    error!("Error writing to server: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to connect to server: {}", e);
            }
        }
    }

    pub async fn listen(&self) -> io::Result<()> {
        info!("Listening on: {}", self.listener.local_addr()?);

        loop {
            let (mut stream, peer_addr) = self.listener.accept().await?;
            info!("New connection from: {}", peer_addr);

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
                                    let message: Message = received.into();
                                    info!("Received: {:?} from {}", message, peer_addr);

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
                        // You could add other asynchronous tasks here that might interact with this connection
                        // For example, a timer or a channel receiver.
                    }
                }
            });
        }
    }
}
