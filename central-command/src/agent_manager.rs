/// The `AgentManager` struct is responsible for managing connections to agents,
/// dispatching jobs, and maintaining the state of connected agents in a distributed system.
///
/// # Responsibilities
/// - Maintains a map of currently connected agents and their TCP streams.
/// - Periodically fetches agent information from a database and attempts to connect to new agents.
/// - Pings connected agents to ensure they are still reachable, removing any that are unreachable.
/// - Dispatches jobs to agents based on job requirements and agent availability.
/// - Updates job status and tracks which agents are running which jobs in the database.
///
/// # Key Methods
/// - `new`: Creates a new `AgentManager` with the provided datastore.
/// - `fetch_database_agents`: Retrieves all agents from the database and converts them into `ConnectedAgent` instances.
/// - `check_for_unconnected_agents`: Checks for agents in the database that are not currently connected and attempts to connect to them.
/// - `fetch_unconnected_agents`: Returns a list of agents from the database that are not currently connected.
/// - `connect_unconnected_agents`: Attempts to establish TCP connections to a list of unconnected agents.
/// - `ping_existing_agents`: Sends a ping message to each connected agent and removes those that are unreachable.
/// - `run_job`: Dispatches a job to the required agents and updates the job's running state in the database.
/// - `get_jobs_to_run`: Retrieves jobs from the database that are ready to run and updates their status.
/// - `add_agent_to_running_job`: Updates a job in the database to include an agent in its running list.
/// - `start`: Launches background tasks to periodically check for new agents, ping existing agents, connect to unconnected agents, and dispatch jobs.
///
/// # Usage
/// Create an `AgentManager` instance and call `start` to begin managing agents and dispatching jobs.
///
/// # Example
/// ```rust
/// let datastore = Arc::new(Datastore::new(...));
/// let agent_manager = AgentManager::new(datastore).await;
/// agent_manager.start().await;
/// ```
///
/// # Thread Safety
/// The `AgentManager` uses `tokio::sync::Mutex` and `Arc` to ensure safe concurrent access
/// when used in asynchronous tasks.
///
/// # Errors
/// Most methods return `Result` types and log errors using the `tracing` crate.
/// Errors are handled gracefully to ensure the manager continues running.
use bson::{DateTime, Document, doc};
use futures::stream::TryStreamExt;
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

use core_logic::communications::{DispatchJob, Message, MessageError};
use core_logic::datastore::{
    Datastore,
    agents::{AgentV1, Status as AgentStatus},
    jobs::{JobV1, Status},
};
use tokio::io::AsyncReadExt;

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct ConnectedAgent {
    name: String,
    address: SocketAddr,
}

impl TryFrom<AgentV1> for ConnectedAgent {
    type Error = std::io::Error;

    fn try_from(agent: AgentV1) -> Result<Self, Self::Error> {
        let addr = format!("{}:{}", agent.hostname, agent.port);
        let socket_addr = addr.to_socket_addrs()?.next().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid address")
        })?;
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
            .filter(|agent| {
                !self.connected_agents.keys().any(|connected_agent| {
                    connected_agent.address.port() == agent.address.port()
                        && connected_agent.address.ip() == agent.address.ip()
                })
            })
            .cloned()
            .collect()
    }
    /// Connect to unconnected agents
    /// Attempts to connect to each unconnected agent and adds them to the `connected_agents` map
    async fn connect_unconnected_agents(&mut self, unconnected_agents: Vec<ConnectedAgent>) {
        let datastore = self.datastore.clone();
        for agent in unconnected_agents {
            match TcpStream::connect(agent.address).await {
                Ok(stream) => {
                    info!("Connected to agent {}!", agent.address);
                    self.connected_agents.insert(agent, stream);
                }
                Err(e) => {
                    error!("Error connecting to agent {}: {}", agent.address, e);
                    if let Err(err) = Self::update_agent_offline(datastore.clone(), &agent).await {
                        error!("Failed to update agent {} to offline: {}", agent.name, err);
                    }
                }
            }
        }
    }

    /// Check if connected agents are still reachable
    /// This function sends a ping message to each connected agent and removes those that are unreachable
    async fn ping_existing_agents(&mut self) {
        let mut agents_to_remove = Vec::new();

        let datastore = self.datastore.clone();

        for (agent, stream) in self.connected_agents.iter_mut() {
            debug!("Pinging agent {}!", agent.address);

            let message = Message::Ping;
            match Self::write_to_agent(stream, &message).await {
                Ok(_) => {
                    debug!("Agent {} is reachable.", agent.address);
                }
                Err(e) => {
                    error!("Failed to ping agent {}: {}", agent.address, e);
                    agents_to_remove.push(agent.clone());
                    continue; // Skip to the next agent
                }
            }
            match Self::update_agent_online(datastore.clone(), agent).await {
                Ok(_) => {
                    debug!("Updated agent {} to online status.", agent.name);
                }
                Err(e) => {
                    error!("Failed to update agent {} to online: {}", agent.name, e);
                }
            }
        }

        for agent in agents_to_remove {
            debug!("Removing agent {} due to failed ping.", agent.address);
            // Update the agent's status to offline in the database
            if let Err(e) = Self::update_agent_offline(datastore.clone(), &agent).await {
                error!("Failed to update agent {} to offline: {}", agent.name, e);
            }
            self.connected_agents.remove(&agent);
        }
    }

    async fn update_agent_offline(
        datastore: Arc<Datastore>,
        agent: &ConnectedAgent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let collection = datastore.get_collection::<AgentV1>("agents").await?;
        let filter = doc! { "name": &agent.name };
        let update = doc! {
            "$set": {
                //"last_ping": DateTime::now(),
                "status": AgentStatus::Offline as i32, // Update status to Offline
            }
        };
        collection.update_one(filter, update).await?;
        Ok(())
    }

    async fn update_agent_online(
        datastore: Arc<Datastore>,
        agent: &ConnectedAgent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let collection = datastore.get_collection::<AgentV1>("agents").await?;
        let filter = doc! { "name": &agent.name };
        let update = doc! {
            "$set": {
            "last_ping": DateTime::now(),
            "status": AgentStatus::Online as i32, // Update status to Online
            }
        };
        collection.update_one(filter, update).await?;
        Ok(())
    }

    /// Run a job
    /// This function sends a `DispatchJob` message to each required agent and updates the job's `agents_running` list.
    async fn run_job(&mut self, job: &JobV1) -> Result<(), Box<dyn std::error::Error>> {
        let datastore = self.datastore.clone();
        let agents_to_run: &HashSet<String> = &job.agents_required.iter().cloned().collect();

        for (agent, stream) in self.connected_agents.iter_mut() {
            if !agents_to_run.contains(&agent.name) {
                continue;
            }

            let dispatch_job = DispatchJob {
                job_name: job.name.clone(),
                command: job.command.clone(),
                args: job.args.join(" "),
                valid_return_codes: Some(job.valid_return_codes.clone()),
                agent_name: Some(agent.name.clone()),
            };
            let message = Message::DispatchJob(dispatch_job);

            if let Err(e) = Self::write_to_agent(stream, &message).await {
                error!("Failed to dispatch job to agent {}: {}", agent.address, e);
                continue;
            }
            Self::add_agent_to_running_job(datastore.clone(), job, &agent.name).await?;
            debug!("Dispatched job to agent {}: {:?}", agent.address, message);
        }

        Ok(())
    }

    async fn write_to_agent(stream: &mut TcpStream, message: &Message) -> Result<(), MessageError> {
        match message.clone().tcp_write(stream).await {
            Ok(_) => {
                // Wait for a response from the agent
                let mut buf = [0u8; 2]; // Adjust buffer size as needed for your protocol
                match stream.read_exact(&mut buf).await {
                    Ok(_) if &buf == b"OK" => Ok(()),
                    _ => Err(MessageError::AcknowledgeError(
                        "Failed to receive acknowledgment from agent".to_string(),
                    )),
                }
            }
            Err(e) => {
                error!("Error writing to agent: {}", e);
                Err(e.into())
            }
        }
    }

    /// Get jobs to run
    /// This function retrieves jobs from the database that are ready to run (status 0 and next_run < current time)
    /// It updates their status to 1 (running) and returns the jobs that are now running without agents.
    pub async fn get_jobs_to_run(
        datastore: Arc<Datastore>,
        connected_agents: Vec<String>,
    ) -> Result<Vec<JobV1>, Box<dyn std::error::Error>> {
        let timestamp = DateTime::now().to_chrono().timestamp();
        let collection = datastore.clone().get_collection::<JobV1>("jobs").await?;
        // Filter for jobs with status 0 and next_run < current time
        let filter = doc! {
            "$and": [
                { "status": Status::Pending }, // Jobs with status equal to 0
                { "next_run": { "$lt": timestamp } },  // Jobs where next_run is LESS THAN current_utc_time
                { "agents_running": [] }, // Jobs that are not currently running with agents
                { "agents_required": { "$in": connected_agents } }
            ]
        };
        let update = doc! {
            "$set": {
                "status": Status::Running
            },
        };
        // Update the status of the jobs to 1 (running)
        let _ = collection.update_many(filter, update).await?;
        // Now fetch the jobs that are ready to run
        let post_filter = doc! {
            "$and": [
                { "status": Status::Running  }, // Jobs with status equal to 1
                { "agents_running": [] }
            ]
        };
        // Fetch the jobs that are now running without agents
        let mut cursor = collection.find(post_filter).await?;
        let mut jobs = vec![];
        while let Some(job) = cursor.try_next().await? {
            jobs.push(job);
        }
        Ok(jobs)
    }

    /// Add an agent to the running job
    /// This function updates the job in the database to include the agent in the `agents_running` list
    /// It checks if the agent is already in the list to avoid duplicates.
    /// Returns `Ok(())` if the agent was added successfully, or an error if the update failed.
    pub async fn add_agent_to_running_job(
        datastore: Arc<Datastore>,
        job: &JobV1,
        agent_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !job.agents_running.contains(&agent_name.to_string()) {
            let mut agents_running = job.agents_running.clone();
            agents_running.push(agent_name.to_string());
            let collection = datastore.get_collection::<JobV1>("jobs").await?;
            let filter = doc! { "_id": job.id };
            let update = doc! { "$set": { "agents_running": agents_running } };
            collection.update_one(filter, update).await?;
        }
        Ok(())
    }

    /// Check if connected agents are still reachable
    pub async fn start(self) {
        const AGENT_PING_KEEP_ALIVE: u64 = 5; // Interval to ping agents
        const UNCONNECT_CHECK_INTERVAL_SECONDS: u64 = 5; // Interval to check for unconnected agents
        const JOB_DISPATCH_INTERVAL_SECONDS: u64 = 1; // Interval to check for jobs to dispatch

        let manager = Arc::new(Mutex::new(self)); // Ownership of `self` is moved here

        // Pings Agents
        let manager_clone = manager.clone();
        spawn(async move {
            loop {
                let mut manager_lock = manager_clone.lock().await;
                manager_lock.ping_existing_agents().await;
                drop(manager_lock); // Explicitly drop the lock to avoid holding it while sleeping
                sleep(Duration::from_secs(AGENT_PING_KEEP_ALIVE)).await;
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
                let mut manager_lock = manager_clone.lock().await;
                debug!("Checking for jobs to dispatch...");
                let connected_agents = manager_lock
                    .connected_agents
                    .keys()
                    .map(|a| a.name.clone())
                    .collect::<Vec<_>>();
                let data_store = manager_lock.datastore.clone();
                let jobs_to_run =
                    match AgentManager::get_jobs_to_run(data_store, connected_agents).await {
                        Ok(jobs) => jobs,
                        Err(e) => {
                            error!("Error fetching jobs: {}", e);
                            continue; // Skip this iteration on error
                        }
                    };
                for job in jobs_to_run.iter() {
                    info!("Running job: {:?}", job);
                    let _ = manager_lock.run_job(job).await;
                }
                drop(manager_lock); // Explicitly drop the lock to avoid holding it while sleeping
                sleep(Duration::from_secs(JOB_DISPATCH_INTERVAL_SECONDS)).await;
            }
        });
    }
}
