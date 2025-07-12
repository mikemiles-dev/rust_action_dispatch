use bson::{Bson, oid::ObjectId};
use mongodb::{
    Collection,
    bson::{DateTime, Document, doc},
};
use serde::{Deserialize, Serialize};

use tracing::error;

use std::error::Error;

use crate::datastore::Datastore;
use crate::messages::RegisterAgent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
#[serde(from = "i32")]
#[serde(into = "i32")]
pub enum Status {
    Offline = 0,
    Online = 1,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct AgentV1 {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub hostname: String,
    pub last_ping: DateTime,
    pub status: Status,
    pub port: u16,
    pub version: u32,
}

impl Default for AgentV1 {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            hostname: String::new(),
            last_ping: DateTime::from_millis(0),
            status: Status::Offline,
            port: 0,
            version: 1,
        }
    }
}

impl From<Status> for i32 {
    fn from(status: Status) -> Self {
        status as i32
    }
}

impl From<i32> for Status {
    fn from(value: i32) -> Self {
        match value {
            0 => Status::Offline,
            1 => Status::Online,
            _ => {
                error!("Warning: Unknown Status value encountered: {}", value);
                Status::Offline // Default to Offline for unknown values
            }
        }
    }
}

impl From<Status> for Bson {
    fn from(status: Status) -> Self {
        Bson::Int32(status as i32)
    }
}

impl AgentV1 {
    pub async fn create_indicies(collection: &Collection<Document>) -> Result<(), Box<dyn Error>> {
        let index_doc = doc! { "hostname": 1, "port": 1 };
        Datastore::create_unique_index(collection, index_doc).await?;
        let index_doc = doc! { "name": 1, };
        Datastore::create_unique_index(collection, index_doc).await?;

        Ok(())
    }
}

impl std::fmt::Display for AgentV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AgentV1 {{ id: {:?}, name: {}, hostname: {}, port: {}, version: {} }}",
            self.id, self.name, self.hostname, self.port, self.version
        )
    }
}

impl From<RegisterAgent> for AgentV1 {
    fn from(register_agent: RegisterAgent) -> Self {
        Self {
            id: None,
            name: register_agent.name,
            hostname: register_agent.hostname,
            last_ping: DateTime::from_millis(0), // Default to 0, will be updated on next ping
            status: Status::Offline,             // Default to Offline, will be updated on next ping
            port: register_agent.port,
            version: 1,
        }
    }
}
