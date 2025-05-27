use bson::oid::ObjectId;
use mongodb::bson::{Document, doc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct JobV1 {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub next_run: u64,
    pub status: u8,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<String>,
    pub cwd: String,
    pub timeout: u32,
    pub retries: u32,
    pub agent_name: String,
    pub version: u32,
}

impl JobV1 {
    pub async fn create_indicies(
        collection: &mongodb::Collection<Document>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let index_doc = doc! { "name": 1, };
        crate::datastore::Datastore::create_unique_index(collection, index_doc).await?;

        Ok(())
    }
}
