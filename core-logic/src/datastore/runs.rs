use bson::{DateTime, oid::ObjectId};
use mongodb::bson::{Document, doc};
use serde::{Deserialize, Serialize};

use std::error::Error;

use crate::communications::JobComplete;

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct RunsV1 {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub started_at: DateTime,
    pub completed_at: DateTime,
    pub job_name: String,
    pub agent_name: String,
    pub return_code: i32,
}

impl RunsV1 {
    pub async fn insert_entry(&self, db: &mongodb::Database) -> Result<(), Box<dyn Error>> {
        let runs_collection = db.collection::<Document>("runs");
        let doc = bson::to_document(self)?;
        runs_collection.insert_one(doc).await?;
        Ok(())
    }
}

impl From<JobComplete> for RunsV1 {
    fn from(job_complete: JobComplete) -> Self {
        Self {
            id: None,
            started_at: DateTime::from_millis(job_complete.started_at),
            completed_at: DateTime::from_millis(job_complete.completed_at),
            job_name: job_complete.job_name,
            agent_name: job_complete.agent_name,
            return_code: job_complete.return_code,
        }
    }
}
