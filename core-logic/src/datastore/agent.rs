use serde::{Deserialize, Serialize};

use crate::communications::RegisterAgent;

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentV1 {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub hostname: String,
    pub port: u16,
    pub version: u32,
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
