use bson::Document;
use futures::stream::TryStreamExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{debug, error, info};

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use core_logic::communications::Message;
use core_logic::datastore::{Datastore, agents::AgentV1};

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ConnectedAgent {
    name: String,
    address: SocketAddr,
}

impl TryFrom<AgentV1> for ConnectedAgent {
    type Error = std::io::Error;

    fn try_from(agent: AgentV1) -> Result<Self, Self::Error> {
        let addr = format!("{}:{}", agent.hostname, agent.port);
        let mut socket_addr = addr.to_socket_addrs()?;
        let socket_addr = socket_addr.next().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid address",
        ))?;
        Ok(ConnectedAgent {
            name: agent.name,
            address: socket_addr,
        })
    }
}

#[derive(Debug)]
pub struct AgentManager {
    datastore: Arc<Datastore>,
    connected_agents: HashMap<ConnectedAgent, TcpStream>,
}

impl AgentManager {
    pub async fn new(datastore: Arc<Datastore>) -> Self {
        Self {
            datastore,
            connected_agents: HashMap::new(),
        }
    }

    /// Fetch agents from the database and convert them to ConnectedAgent
    async fn fetch_agents(
        &mut self,
    ) -> Result<HashSet<ConnectedAgent>, Box<dyn std::error::Error>> {
        let agents = self.fetch_agents_from_db().await?;
        let new_agents = self.convert_to_connected_agents(agents).await?;
        Ok(new_agents)
    }

    /// Fetch agents from the database
    async fn fetch_agents_from_db(&self) -> Result<Vec<AgentV1>, Box<dyn std::error::Error>> {
        let collection = self.datastore.get_collection::<AgentV1>("agents").await?;
        let filter = Document::new();
        let mut cursor = collection.find(filter, None).await?;
        let mut agents = vec![];
        while let Some(agent) = cursor.try_next().await? {
            agents.push(agent);
        }
        Ok(agents)
    }

    /// Convert agents to ConnectedAgent
    async fn convert_to_connected_agents(
        &self,
        agents: Vec<AgentV1>,
    ) -> Result<HashSet<ConnectedAgent>, Box<dyn std::error::Error>> {
        let mut new_agents = HashSet::new();
        for agent in agents.iter() {
            match self.create_connected_agent(agent).await {
                Ok(agent) => {
                    new_agents.insert(agent);
                }
                Err(e) => {
                    error!("Unable to connect to agent {}: {:?}", agent, e);
                }
            }
        }
        Ok(new_agents)
    }

    /// Create a ConnectedAgent from an AgentV1
    async fn create_connected_agent(
        &self,
        agent: &AgentV1,
    ) -> Result<ConnectedAgent, Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", agent.hostname, agent.port);
        let mut socket_addr = addr.to_socket_addrs()?;
        let socket_addr = socket_addr.next().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid address")
        })?;
        Ok(ConnectedAgent {
            name: agent.name.clone(),
            address: socket_addr,
        })
    }

    /// Check for unconnected agents and connect to them
    async fn check_unconnected(&mut self) {
        debug!("Checking for unconnected agents...");
        let unconnected_agents = self.get_unconnected().await;
        if !unconnected_agents.is_empty() {
            info!(
                "Agents that are not connected: {:?}",
                unconnected_agents
                    .iter()
                    .map(|a| a.address)
                    .collect::<Vec<_>>()
            );
            self.connect_unconnected(unconnected_agents).await;
        }
    }

    /// Get unconnected agents
    async fn get_unconnected(&mut self) -> Vec<ConnectedAgent> {
        let fetched_agents = match self.fetch_agents().await {
            Ok(agents) => agents,
            Err(e) => {
                error!("Error fetching agents: {}", e);
                return Vec::new();
            }
        };
        debug!("Fetched agents: {:?}", fetched_agents);

        fetched_agents
            .iter()
            .filter(|agent| !self.connected_agents.contains_key(agent))
            .cloned()
            .collect()
    }

    /// Connect to unconnected agents
    async fn connect_unconnected(&mut self, unconnected_agents: Vec<ConnectedAgent>) {
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

    /// Check if connected agents are still reachable
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

    /// Check if connected agents are still reachable
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
                debug!("Checking for new agents in the database...");
                let _ = self.fetch_agents().await;
                info!(
                    "Agents that are connected: {{{}}}",
                    self.connected_agents
                        .keys()
                        .map(|a| format!("{}:{}", a.name, a.address))
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
