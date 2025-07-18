//! This module provides the core logic for interacting with the MongoDB datastore,
//! including initialization, index creation, and collection access for the application.
//!
//! # Modules
//! - `agents`: Contains logic and data structures related to agents.
//! - `jobs`: Contains logic and data structures related to jobs.
//!
//! # Structs
//! - [`Datastore`]: Represents a connection to the MongoDB database and provides methods
//!   for managing collections and indices.
//!
//! # Enums
//! - [`DataStoreTypes`]: Enum representing different types of data stored in the datastore.
//!
//! # Constants
//! - `MONGODB_URI`: Default MongoDB connection string used if the environment variable is not set.
//!
//! # Usage
//! - Use [`Datastore::try_new`] to initialize a new datastore connection.
//! - Use [`Datastore::get_collection`] to access specific collections.
//! - Use [`Datastore::create_unique_index`] to create unique indices on collections.
//!
//! # Errors
//! - Most methods return a `Result` type and may return errors related to MongoDB operations.
//!
//! # Logging
//! - Uses the `tracing` crate for logging connection and configuration information.
pub mod agents;
pub mod jobs;
pub mod runs;

use mongodb::{
    Client, Collection, IndexModel,
    bson::Document,
    error::Error as MongoError,
    options::{ClientOptions, IndexOptions},
};

use std::env;
use std::error::Error;

use tracing::{info, warn};

use agents::AgentV1;
use jobs::JobV1;

const MONGODB_URI: &str = "mongodb://localhost:27017";
const DATABASE_NAME: &str = "rust-action-dispatch";

pub enum DataStoreTypes {
    Agent(AgentV1),
}

#[derive(Debug)]
pub struct Datastore {
    pub client: Client,
}

impl Datastore {
    pub async fn create_unique_index(
        collection: &Collection<Document>,
        doc: Document,
    ) -> Result<(), Box<dyn Error>> {
        let index_options = IndexOptions::builder().unique(true).build();
        let index_model = IndexModel::builder()
            .keys(doc) // 1 for ascending, -1 for descending
            .options(index_options)
            .build();

        collection.create_index(index_model).await?;

        Ok(())
    }
}

impl Datastore {
    pub fn get_database(&self) -> mongodb::Database {
        self.client.database(DATABASE_NAME)
    }

    pub async fn try_new() -> Result<Self, MongoError> {
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

        let options = ClientOptions::parse(&client_uri).await?;

        let client = Client::with_options(options)?;
        let db = client.database(DATABASE_NAME);

        let agents = db.collection::<bson::Document>("agents");
        AgentV1::create_indicies(&agents)
            .await
            .expect("Failed to create mongodb indices");
        let jobs = db.collection::<bson::Document>("jobs");
        JobV1::create_indicies(&jobs)
            .await
            .expect("Failed to create mongodb indices");

        Ok(Datastore { client })
    }

    pub async fn get_collection<T: Sync + std::marker::Send + serde::de::DeserializeOwned>(
        &self,
        collection_name: &str,
    ) -> Result<Collection<T>, Box<dyn Error>> {
        let collection = self.get_database().collection::<T>(collection_name);
        Ok(collection)
    }
}
