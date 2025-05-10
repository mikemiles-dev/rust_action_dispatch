mod agent;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use tokio::sync::mpsc;
use uuid::Uuid;

use std::error::Error;
use std::net::SocketAddr;

use tracing::{error, info};

const SERVER_ADDRESS: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up tracing subscriber for logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set the minimum level to display
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    // info!("Binding on address: {}", SERVER_ADDRESS);
    // let listener = TcpListener::bind(SERVER_ADDRESS).await?;
    // info!("Listening on: {}", SERVER_ADDRESS);

    // let (tx, mut rx) = mpsc::channel::<String>(32);

    // // Spawn a task to handle incoming connections
    // spawn(async move {
    //     while let Ok((stream, addr)) = listener.accept().await {
    //         info!("Accepted connection from: {}", addr);
    //         let tx_clone = tx.clone();
    //         spawn(async move {
    //             if let Err(e) = handle_connection(stream, tx_clone).await {
    //                 error!("Error handling connection from {}: {}", addr, e);
    //             }
    //         });
    //     }
    // });

    // Spawn a task to connect to the server and send data
    spawn(async move {
        AgentManager::default().start().await;
    });

    // Keep the main task alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down.");

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    address: SocketAddr,
}

#[derive(Debug, Default)]
pub struct AgentManager {
    agents: Vec<Agent>,
    connected_agents: Vec<Agent>,
}

impl AgentManager {
    fn populate_agents(&mut self) {
        self.agents = Vec::from([Agent {
            address: "127.0.0.1:8081".parse().unwrap(),
        }]);
    }

    async fn check_unconnected(&mut self) {
        info!("Checking for unconnected agents...");
        let unconnected_agents = self.get_unconnected();
        if !unconnected_agents.is_empty() {
            info!(
                "Found unconnected agents: {:?}",
                unconnected_agents
                    .iter()
                    .map(|a| a.address)
                    .collect::<Vec<_>>()
            );
            self.connect_unconnected(unconnected_agents).await;
        }
    }

    fn get_unconnected(&mut self) -> Vec<Agent> {
        self.agents
            .iter()
            .filter(|agent| !self.connected_agents.contains(agent))
            .cloned()
            .collect()
    }

    async fn connect_unconnected(&mut self, unconnected_agents: Vec<Agent>) {
        for agent in unconnected_agents.into_iter() {
            match TcpStream::connect(agent.address).await {
                Ok(mut stream) => {
                    info!("Connected to agent {}!", agent.address);
                    self.connected_agents.push(agent.clone());
                    stream
                        .write_all("Hello from command!".as_bytes())
                        .await
                        .unwrap();
                }
                Err(e) => {
                    error!("Error connecting to agent {}: {}", agent.address, e);
                }
            }
        }
    }

    async fn start(&mut self) {
        self.populate_agents();

        loop {
            self.check_unconnected().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }
}
