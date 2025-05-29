use bson::{DateTime, Document, doc, oid::ObjectId};
use futures::stream::TryStreamExt;
use mongodb::bson::Bson;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use crate::datastore::Datastore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
#[serde(from = "i32")]
pub enum Status {
    Pending = 0,
    Running = 1,
    Completed = 2,
    Error = 3,
}

// Implementation to convert from i32 to Status
impl From<i32> for Status {
    fn from(value: i32) -> Self {
        match value {
            0 => Status::Pending,
            1 => Status::Running,
            2 => Status::Completed,
            3 => Status::Error,
            _ => {
                // Handle unknown values gracefully (e.g., default to Error or Pending)
                // Or panic if an invalid status is truly an unrecoverable error.
                eprintln!("Warning: Unknown Status value encountered: {}", value);
                Status::Error // Or Status::Pending, or panic!
            }
        }
    }
}

impl From<Status> for Bson {
    fn from(status: Status) -> Self {
        Bson::Int32(status as i32)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobV1 {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub next_run: i64,
    pub status: Status,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<String>,
    pub cwd: String,
    pub timeout: u32,
    pub retries: u32,
    pub valid_return_codes: Vec<i32>,
    pub agents_required: Vec<String>,
    pub agents_running: Vec<String>,
    pub agents_complete: Vec<String>,
}

impl JobV1 {
    pub async fn create_indicies(
        collection: &mongodb::Collection<Document>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let index_doc = doc! { "name": 1, };
        crate::datastore::Datastore::create_unique_index(collection, index_doc).await?;

        Ok(())
    }

    /// Get jobs to run
    /// This function retrieves jobs from the database that are ready to run (status 0 and next_run < current time)
    /// It updates their status to 1 (running) and returns the jobs that are now running without agents.
    pub async fn get_jobs_to_run(
        datastore: Arc<Datastore>,
        connected_agents: Vec<String>,
    ) -> Result<Vec<JobV1>, Box<dyn std::error::Error>> {
        let timestamp = DateTime::now().to_chrono().timestamp();
        let collection = datastore.clone().get_collection::<JobV1>("jobs").await?;
        // Filter for jobs with status 0 and next_run < current time
        let filter = doc! {
            "$and": [
                { "status": Status::Pending }, // Jobs with status equal to 0
                { "next_run": { "$lt": timestamp } },  // Jobs where next_run is LESS THAN current_utc_time
                { "agents_running": [] }, // Jobs that are not currently running with agents
                { "agents_required": { "$in": connected_agents } }
            ]
        };
        let update = doc! {
            "$set": {
                "status": Status::Running
            },
        };
        // Update the status of the jobs to 1 (running)
        let _ = collection.update_many(filter, update).await?;
        // Now fetch the jobs that are ready to run
        let post_filter = doc! {
            "$and": [
                { "status": Status::Running  }, // Jobs with status equal to 1
                { "agents_running": [] }
            ]
        };
        // Fetch the jobs that are now running without agents
        let mut cursor = collection.find(post_filter).await?;
        let mut jobs = vec![];
        while let Some(job) = cursor.try_next().await? {
            jobs.push(job);
        }
        Ok(jobs)
    }

    /// Add an agent to the running job
    /// This function updates the job in the database to include the agent in the `agents_running` list
    /// It checks if the agent is already in the list to avoid duplicates.
    /// Returns `Ok(())` if the agent was added successfully, or an error if the update failed.
    pub async fn add_agent_to_running_job(
        datastore: Arc<Datastore>,
        job: &JobV1,
        agent_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !job.agents_running.contains(&agent_name.to_string()) {
            let mut agents_running = job.agents_running.clone();
            agents_running.push(agent_name.to_string());
            let collection = datastore.get_collection::<JobV1>("jobs").await?;
            let filter = doc! { "_id": job.id };
            let update = doc! { "$set": { "agents_running": agents_running } };
            collection.update_one(filter, update).await?;
        }
        Ok(())
    }
}
