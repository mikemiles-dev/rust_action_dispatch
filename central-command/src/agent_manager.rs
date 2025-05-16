use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{debug, error, info};

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use core_logic::communications::{Message, RegisterAgent};
use core_logic::datastore::Datastore;

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Agent {
    name: String,
    address: SocketAddr,
}

#[derive(Debug)]
pub struct AgentManager {
    datastore: Arc<Datastore>,
    agents: HashSet<Agent>,
    connected_agents: HashMap<Agent, TcpStream>,
}

impl AgentManager {
    pub async fn new(datastore: Arc<Datastore>) -> Self {
        Self {
            datastore,
            agents: HashSet::new(),
            connected_agents: HashMap::new(),
        }
    }

    async fn fetch_agents(&mut self) {
        // let agents = DB_AGENTS.read().await;
        // let mut new_agents = HashSet::new();
        // for register_agent in agents.iter() {
        //     let addr = format!("{}:{}", register_agent.hostname, register_agent.port);
        //     let mut socket_addr = match addr.to_socket_addrs() {
        //         Ok(s) => s,
        //         Err(_) => {
        //             error!("Inalid agent! {:?}", register_agent);
        //             continue;
        //         }
        //     };
        //     let socket_addr = match socket_addr.next() {
        //         Some(addr) => addr,
        //         None => {
        //             error!("Invalid agent! {:?}", register_agent);
        //             continue;
        //         }
        //     };
        //     new_agents.insert(Agent {
        //         name: register_agent.name.clone(),
        //         address: socket_addr,
        //     });
        // }
        // self.agents = new_agents;
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

            let message: Vec<u8> = match Message::Ping.try_into() {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    continue;
                }
            };

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

    pub async fn start(&mut self) {
        const CONNECT_CHECK_INTERVAL_SECONDS: u64 = 10;
        const UNCONNECT_CHECK_INTERVAL_SECONDS: u64 = 5;
        const AGENT_DB_CHECK_INTERVAL_SECONDS: u64 = 5;

        let mut last_connected_check = Instant::now()
            .checked_sub(Duration::from_secs(CONNECT_CHECK_INTERVAL_SECONDS))
            .unwrap_or(Instant::now());
        let mut last_unconnected_check = Instant::now()
            .checked_sub(Duration::from_secs(UNCONNECT_CHECK_INTERVAL_SECONDS))
            .unwrap_or(Instant::now());
        let mut last_agent_db_check = Instant::now()
            .checked_sub(Duration::from_secs(AGENT_DB_CHECK_INTERVAL_SECONDS))
            .unwrap_or(Instant::now());

        loop {
            if last_agent_db_check.elapsed().as_secs() > AGENT_DB_CHECK_INTERVAL_SECONDS {
                //self.populate_agents().await;
                info!(
                    "Connected agents are: {{{}}}",
                    self.agents
                        .iter()
                        .map(|a| format!("{} => {}", a.name, a.address))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                last_agent_db_check = Instant::now();
            }

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
