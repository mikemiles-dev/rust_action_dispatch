pub mod agents;
pub mod jobs;

use mongodb::{
    Client, Collection, IndexModel,
    bson::Document,
    error::Error as MongoError,
    options::{ClientOptions, IndexOptions, ResolverConfig, Tls, TlsOptions},
};

use std::env;
use std::error::Error;
use std::path::PathBuf;

use tracing::{info, warn};

use agents::AgentV1;
use jobs::JobV1;

const MONGODB_URI: &str = "mongodb://localhost:27017";

pub enum DataStoreTypes {
    Agent(AgentV1),
}

#[derive(Debug)]
pub struct Datastore {
    pub client: Client,
}

impl Datastore {
    pub async fn create_indicies(
        collection: &Collection<Document>,
        doc: Document,
    ) -> Result<(), Box<dyn Error>> {
        let index_options = IndexOptions::builder().unique(true).build();
        let index_model = IndexModel::builder()
            .keys(doc) // 1 for ascending, -1 for descending
            .options(index_options)
            .build();

        collection.create_index(index_model, None).await?;

        Ok(())
    }
}

impl Datastore {
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

        let mongo_cert_crt = env::var("MONGO_CRT").unwrap_or("../keys/server.crt".to_string());
        let ca_file_path = PathBuf::from(mongo_cert_crt); // Path to your CA certificate (self-signed acts as CA)
        let mongo_cert_pem = env::var("MONGO_PEM").unwrap_or("../keys/server.pem".to_string());
        let cert_key_file_path = PathBuf::from(mongo_cert_pem); // Path to your combined certificate and private key

        let mut options =
            ClientOptions::parse_with_resolver_config(&client_uri, ResolverConfig::cloudflare())
                .await?;

        let tls_options = TlsOptions::builder()
            .ca_file_path(Some(ca_file_path))
            .cert_key_file_path(Some(cert_key_file_path))
            // If your private key in server.pem is password-protected, you'd add:
            // .tls_certificate_key_file_password(Some("your_password".to_string().into_bytes()))
            .build();

        options.tls = Some(Tls::Enabled(tls_options));

        let client = Client::with_options(options)?;
        let db = client.database("rust-action-dispatch");

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

    pub async fn get_collection<T: serde::de::DeserializeOwned>(
        &self,
        collection_name: &str,
    ) -> Result<Collection<T>, Box<dyn Error>> {
        let collection = self
            .client
            .database("rust-action-dispatch")
            .collection::<T>(collection_name);
        Ok(collection)
    }
}
