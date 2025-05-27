use bson::{Document, doc};
use futures::stream::TryStreamExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, error, info};

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use core_logic::communications::{DispatchJob, Message};
use core_logic::datastore::{Datastore, agents::AgentV1, jobs::JobV1};

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

    /// Fetch agents from the database
    /// This function retrieves all agents from the database and converts them into `ConnectedAgent` instances
    async fn fetch_database_agents(
        &self,
    ) -> Result<HashSet<ConnectedAgent>, Box<dyn std::error::Error>> {
        let collection = self.datastore.get_collection::<AgentV1>("agents").await?;
        let filter = Document::new();
        let mut cursor = collection.find(filter).await?;
        let mut agents = vec![];
        while let Some(agent) = cursor.try_next().await? {
            agents.push(agent);
        }
        let agents: HashSet<ConnectedAgent> = agents
            .iter()
            .filter_map(|agent| agent.clone().try_into().ok())
            .collect();
        Ok(agents)
    }

    /// Check for unconnected agents and connect to them.
    /// This function will periodically check for agents that are not connected
    async fn check_for_unconnected_agents(&mut self) {
        debug!("Checking for unconnected agents...");
        let unconnected_agents = self.fetch_unconnected_agents().await;
        if !unconnected_agents.is_empty() {
            info!(
                "Agents that are not connected: {:?}",
                unconnected_agents
                    .iter()
                    .map(|a| a.address)
                    .collect::<Vec<_>>()
            );
            self.connect_unconnected_agents(unconnected_agents).await;
        }
    }

    /// Get unconnected agents.
    /// Fetch agents from the database and filter out those that are already connected
    async fn fetch_unconnected_agents(&mut self) -> Vec<ConnectedAgent> {
        let fetched_agents = match self.fetch_database_agents().await {
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
    // This function attempts to connect to each unconnected agent and adds them to the `connected_agents` map
    async fn connect_unconnected_agents(&mut self, unconnected_agents: Vec<ConnectedAgent>) {
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
    /// This function sends a ping message to each connected agent and removes those that are unreachable
    async fn ping_existing_agents(&mut self) {
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

    pub async fn get_jobs_to_run(&self) -> Result<Vec<JobV1>, Box<dyn std::error::Error>> {
        let collection = self.datastore.get_collection::<JobV1>("jobs").await?;
        let mut filter = Document::new();
        //filter.insert("status", 0); // Assuming 0 is the status for jobs to run
        //filter.insert("next_run", doc! { "$lte": bson::DateTime::now() });
        let mut cursor = collection.find(filter).await?;
        let mut jobs = vec![];
        while let Some(job) = cursor.try_next().await? {
            jobs.push(job);
        }
        Ok(jobs)
    }

    async fn dispatch_job_to_agent(
        &mut self,
        job: DispatchJob,
        agent_name: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (_, stream) = self
            .connected_agents
            .iter_mut()
            .find(|(agent, _)| agent.name == agent_name)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Agent {} not found", agent_name),
                )
            })?;

        let message: Vec<u8> = match Message::DispatchJob(job).try_into() {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to serialize message: {}", e);
                return Err(Box::new(e));
            }
        };

        stream.write_all(&message).await?;
        Ok(())
    }

    /// Check if connected agents are still reachable
    pub async fn start(self) {
        const CONNECT_CHECK_INTERVAL_SECONDS: u64 = 1;
        const UNCONNECT_CHECK_INTERVAL_SECONDS: u64 = 1;
        const AGENT_DB_CHECK_INTERVAL_SECONDS: u64 = 1;

        let manager = Arc::new(Mutex::new(self)); // Ownership of `self` is moved here

        // Spawn a task to periodically check for new agents in the database
        let manager_clone = manager.clone();
        spawn(async move {
            loop {
                let manager_lock = manager_clone.lock().await;
                debug!("Checking for new agents in the database...");
                if let Err(fetch_agents_error) = manager_lock.fetch_database_agents().await {
                    error!("Error fetching agents: {}", fetch_agents_error);
                }
                info!(
                    "Agents that are connected: {{{}}}",
                    manager_lock
                        .connected_agents
                        .keys()
                        .take(100) // Limit to 100 for logging
                        .map(|a| format!("{}:{}", a.name, a.address))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                drop(manager_lock); // Explicitly drop the lock to avoid holding it while sleeping
                sleep(Duration::from_secs(AGENT_DB_CHECK_INTERVAL_SECONDS)).await;
            }
        });

        // Spawn a task to periodically check for unconnected agents
        let manager_clone = manager.clone();
        spawn(async move {
            loop {
                let mut manager_lock = manager_clone.lock().await;
                manager_lock.ping_existing_agents().await;
                drop(manager_lock); // Explicitly drop the lock to avoid holding it while sleeping
                sleep(Duration::from_secs(CONNECT_CHECK_INTERVAL_SECONDS)).await;
            }
        });

        // Spawn a task to periodically check for unconnected agents
        let manager_clone = manager.clone();
        spawn(async move {
            loop {
                let mut manager_lock = manager_clone.lock().await;
                manager_lock.check_for_unconnected_agents().await;
                drop(manager_lock); // Explicitly drop the lock to avoid holding it while sleeping
                sleep(Duration::from_secs(UNCONNECT_CHECK_INTERVAL_SECONDS)).await;
            }
        });

        // Spawn a task to periodically check for jobs to dispatch
        let manager_clone = manager.clone();
        spawn(async move {
            loop {
                let manager_lock = manager_clone.lock().await;
                debug!("Checking for jobs to dispatch...");
                let jobs_to_run = manager_lock.get_jobs_to_run().await.unwrap();
                info!(
                    "Jobs to run: {}",
                    jobs_to_run
                        .iter()
                        .map(|job| job.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                drop(manager_lock); // Explicitly drop the lock to avoid holding it while sleeping
                sleep(Duration::from_secs(1)).await;
            }
        });
    }
}
