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
    name: String,
    pub hostname: String,
    pub port: u16,
    pub version: u32,
}

impl AgentV1 {
    pub async fn create_indicies(
        collection: &mongodb::Collection<Document>,
    ) -> Result<(), Box<dyn Error>> {
        let index_doc = doc! { "hostname": 1, "port": 1 };
        Datastore::create_indicies(collection, index_doc).await?;
        let index_doc = doc! { "name": 1, };
        Datastore::create_indicies(collection, index_doc).await?;

        Ok(())
    }
}

impl From<RegisterAgent> for AgentV1 {
    fn from(register_agent: RegisterAgent) -> Self {
        Self {
            id: None,
            name: register_agent.name,
            hostname: register_agent.hostname,
            port: register_agent.port,
            version: 1,
        }
    }
}
