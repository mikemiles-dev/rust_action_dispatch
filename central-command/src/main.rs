mod agent;

use core_logic::communications::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use tokio::sync::mpsc;
use uuid::Uuid;

use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tracing::{debug, error, info};

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

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Agent {
    address: SocketAddr,
}

#[derive(Debug, Default)]
pub struct AgentManager {
    agents: Vec<Agent>,
    connected_agents: HashMap<Agent, TcpStream>,
}

impl AgentManager {
    fn populate_agents(&mut self) {
        self.agents = Vec::from([Agent {
            address: "127.0.0.1:8081".parse().unwrap(),
        }]);
    }

    async fn check_unconnected(&mut self) {
        debug!("Checking for unconnected agents...");
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
            .filter(|agent| !self.connected_agents.contains_key(agent))
            .cloned()
            .collect()
    }

    async fn connect_unconnected(&mut self, unconnected_agents: Vec<Agent>) {
        for agent in unconnected_agents.into_iter() {
            match TcpStream::connect(agent.address).await {
                Ok(stream) => {
                    info!("Connected to agent {}!", agent.address);
                    self.connected_agents.insert(agent.clone(), stream);
                }
                Err(e) => {
                    error!("Error connecting to agent {}: {}", agent.address, e);
                }
            }
        }
    }

    async fn check_connected(&mut self) {
        let mut agents_to_remove = Vec::new();

        for (agent, stream) in self.connected_agents.iter_mut() {
            debug!("Pinging agent {}!", agent.address);
            let message: Vec<u8> = Message::Ping.into();
            match stream.write_all(&message).await {
                Ok(_) => {
                    debug!("Pinged agent {} successfully!", agent.address);
                }
                Err(e) => {
                    error!("Error pinging agent {}: {}", agent.address, e);
                    agents_to_remove.push(agent.clone());
                }
            }
        }

        for agent in agents_to_remove {
            self.connected_agents.remove(&agent);
        }
    }

    async fn start(&mut self) {
        const CONNECT_CHECK_INTERVAL_SECONDS: u64 = 10;
        const UNCONNECT_CHECK_INTERVAL_SECONDS: u64 = 5;

        let mut last_connected_check = Instant::now()
            .checked_sub(Duration::from_secs(CONNECT_CHECK_INTERVAL_SECONDS))
            .unwrap_or(Instant::now());
        let mut last_unconnected_check = Instant::now()
            .checked_sub(Duration::from_secs(UNCONNECT_CHECK_INTERVAL_SECONDS))
            .unwrap_or(Instant::now());

        self.populate_agents();

        loop {
            if last_connected_check.elapsed().as_secs() > CONNECT_CHECK_INTERVAL_SECONDS {
                self.check_connected().await;
                last_connected_check = Instant::now();
            }

            if last_unconnected_check.elapsed().as_secs() > UNCONNECT_CHECK_INTERVAL_SECONDS {
                self.check_unconnected().await;
                last_unconnected_check = Instant::now();
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}
