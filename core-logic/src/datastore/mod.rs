pub mod agent;

use mongodb::{
    Client,
    bson::{Document, doc},
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

use futures::StreamExt;

const MONGODB_URI: &str = "mongodb://localhost:27017";

pub enum DataStoreTypes {
    Agent(AgentV1),
}

#[derive(Debug)]
pub struct Datastore {
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
        let db = client.database("rust-action-dispatch");

        // Create a channel to send messages to the datastore:
        let (tx, mut rx) = mpsc::channel::<DataStoreTypes>(100);

        spawn(async move {
            while let Some(message) = rx.recv().await {
                match message {
                    DataStoreTypes::Agent(agent) => {
                        // Handle the agent message
                        info!("ZZZ Received agent: {:?}", agent);
                        let agents = db.collection::<bson::Document>("agents");
                        let bson_agent = bson::to_document(&agent).unwrap();
                        let result = agents.insert_one(bson_agent, None).await;
                        match result {
                            Ok(_) => {
                                info!("Inserted agent: {:?}", agent);
                            }
                            Err(e) => {
                                warn!("Failed to insert agent: {:?}", e);
                            }
                        }

                        let filter = doc! {}; // Empty filter to get all documents
                        let mut cursor = agents.find(filter, None).await.unwrap(); //
                        info!("Agents in the database:");
                        while let Some(doc) = cursor.next().await {
                            match doc {
                                Ok(document) => {
                                    info!("{:?}", document);
                                }
                                Err(e) => {
                                    warn!("Failed to retrieve document: {:?}", e);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(Datastore { sender: tx })
    }
}
