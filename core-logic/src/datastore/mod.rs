pub mod agent;

use mongodb::{
    Client,
    error::Error,
    options::{ClientOptions, ResolverConfig},
};
use std::env;
use tokio::{
    spawn,
    sync::mpsc::{self, Receiver, Sender},
};
use tracing::{info, warn};

use agent::AgentV1;

const MONGODB_URI: &str = "mongodb://localhost:27017";

pub enum DataStoreTypes {
    Agent(AgentV1),
}

#[derive(Debug)]
pub struct Datastore {
    pub client: Client,
    pub sender: Sender<DataStoreTypes>,
}

impl Datastore {
    pub async fn try_new() -> Result<Self, Error> {
        // Load the MongoDB connection string from an environment variable:
        let client_uri = match env::var("MONGODB_URI") {
            Ok(uri) => {
                info!("MONGODB_URI set to {}", uri);
                uri
            }
            Err(_) => {
                warn!("MONGODB_URI not set, using default: {}", MONGODB_URI);
                MONGODB_URI.to_string()
            }
        };
        info!("Connecting to MongoDB at {}", client_uri);
        // A Client is needed to connect to MongoDB:
        // An extra line of code to work around a DNS issue on Windows:
        let options =
            ClientOptions::parse_with_resolver_config(&client_uri, ResolverConfig::cloudflare())
                .await?;
        let client = Client::with_options(options)?;

        // Create a channel to send messages to the datastore:
        let (tx, mut rx) = mpsc::channel::<DataStoreTypes>(100);

        spawn(async move {
            while let Some(message) = rx.recv().await {
                match message {
                    DataStoreTypes::Agent(agent) => {
                        // Handle the agent message
                        info!("ZZZ Received agent: {:?}", agent);
                    }
                }
            }
        });

        Ok(Datastore { client, sender: tx })
    }
}
