use bson::oid::ObjectId;
use mongodb::bson::{Document, doc};
use serde::{Deserialize, Serialize};

use std::error::Error;

use crate::communications::RegisterAgent;
use crate::datastore::Datastore;

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentV1 {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub hostname: String,
    pub port: u16,
    pub version: u32,
}

impl AgentV1 {
    pub async fn create_indicies(
        collection: &mongodb::Collection<Document>,
    ) -> Result<(), Box<dyn Error>> {
        Datastore::create_indicies(collection, "hostname").await?;
        Datastore::create_indicies(collection, "port").await?;

        Ok(())
    }
}

impl From<RegisterAgent> for AgentV1 {
    fn from(register_agent: RegisterAgent) -> Self {
        Self {
            id: None,
            hostname: register_agent.hostname,
            port: register_agent.port,
            version: 1,
        }
    }
}
