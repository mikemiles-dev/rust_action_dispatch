use bson::{DateTime, oid::ObjectId};
use mongodb::bson::{Document, doc};
use serde::{Deserialize, Serialize};

use std::error::Error;

use crate::messages::{JobComplete, JobOutCome};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
#[serde(from = "i32")]
#[serde(into = "i32")]
pub enum Outcome {
    Failure = 0,
    Success = 1,
    Unknown,
}

impl From<Outcome> for i32 {
    fn from(outcome: Outcome) -> Self {
        outcome as i32
    }
}

impl From<i32> for Outcome {
    fn from(value: i32) -> Self {
        match value {
            0 => Outcome::Failure,
            1 => Outcome::Success,
            _ => {
                // Log a warning for unknown outcome
                tracing::error!("Warning: Unknown JobOutCome value encountered: {}", value);
                Outcome::Unknown // Default to Unknown for unknown values
            }
        }
    }
}

impl From<JobOutCome> for Outcome {
    fn from(outcome: JobOutCome) -> Self {
        match outcome {
            JobOutCome::Failure => Outcome::Failure,
            JobOutCome::Success => Outcome::Success,
            JobOutCome::Unknown => Outcome::Unknown,
        }
    }
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct RunsV1 {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub started_at: DateTime,
    pub completed_at: DateTime,
    pub job_name: String,
    pub command: String,
    pub outcome: Outcome,
    pub agent_name: String,
    pub return_code: i32,
    pub output: String,
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
            command: job_complete.command,
            agent_name: job_complete.agent_name,
            outcome: job_complete.outcome.into(),
            return_code: job_complete.return_code,
            output: job_complete.output,
        }
    }
}
