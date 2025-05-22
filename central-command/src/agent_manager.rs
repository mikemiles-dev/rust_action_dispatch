use bson::Document;
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

    pub async fn get_jobs_to_dispatch(
        &self,
    ) -> Result<Vec<DispatchJob>, Box<dyn std::error::Error>> {
        let dispatch_job = DispatchJob {
            job_name: "JOB123".to_string(),
            agent_name: Some("foo2".to_string()),
            command: "/bin/ls".to_string(),
        };
        Ok(vec![dispatch_job])
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
                let mut manager_lock = manager_clone.lock().await;
                debug!("Checking for new agents in the database...");
                if let Err(fetch_agents_error) = manager_lock.fetch_agents().await {
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
                manager_lock.check_connected().await;
                drop(manager_lock); // Explicitly drop the lock to avoid holding it while sleeping
                sleep(Duration::from_secs(CONNECT_CHECK_INTERVAL_SECONDS)).await;
            }
        });

        // Spawn a task to periodically check for unconnected agents
        let manager_clone = manager.clone();
        spawn(async move {
            loop {
                let mut manager_lock = manager_clone.lock().await;
                manager_lock.check_unconnected().await;
                drop(manager_lock); // Explicitly drop the lock to avoid holding it while sleeping
                sleep(Duration::from_secs(UNCONNECT_CHECK_INTERVAL_SECONDS)).await;
            }
        });

        // Spawn a task to periodically check for jobs to dispatch
        let manager_clone = manager.clone();
        spawn(async move {
            loop {
                let mut manager_lock = manager_clone.lock().await;
                let jobs_to_dispatch = match manager_lock.get_jobs_to_dispatch().await {
                    Ok(jobs) => jobs,
                    Err(e) => {
                        error!("Error getting jobs to dispatch: {}", e);
                        continue;
                    }
                };

                if jobs_to_dispatch.is_empty() {
                    debug!("No jobs to dispatch.");
                } else {
                    for job in jobs_to_dispatch.iter() {
                        debug!("Dispatching job: {:?}", job);
                        // Dispatch the job to the appropriate agent
                    }
                }
                for job in jobs_to_dispatch.iter() {
                    debug!("Dispatching job: {:?}", job);
                    match manager_lock
                        .dispatch_job_to_agent(job.clone(), "foo2".to_string())
                        .await
                    {
                        Ok(_) => {
                            debug!("Job dispatched successfully!");
                        }
                        Err(e) => {
                            error!("Error dispatching job: {}", e);
                        }
                    }
                    // Dispatch the job to the appropriate agent
                }
                drop(manager_lock); // Explicitly drop the lock to avoid holding it while sleeping
                sleep(Duration::from_secs(5)).await;
            }
        });
    }
}
