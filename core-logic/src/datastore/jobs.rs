use bson::oid::ObjectId;
use mongodb::bson::Bson;
use mongodb::bson::{Document, doc};
use serde::{Deserialize, Serialize};

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
    pub agents_required: Vec<String>,
    pub agents_running: Vec<String>,
    pub agents_complete: Vec<String>,
}

impl From<JobV1> for Document {
    fn from(job: JobV1) -> Self {
        let mut doc = Document::new();
        doc.insert("_id", job.id.map(Bson::ObjectId));
        doc.insert("name", job.name);
        doc.insert("next_run", Bson::Int64(job.next_run));
        doc.insert("status", job.status as i32);
        doc.insert("description", job.description);
        doc.insert("command", job.command);
        doc.insert("args", job.args);
        doc.insert("env", job.env);
        doc.insert("cwd", job.cwd);
        doc.insert("timeout", job.timeout);
        doc.insert("retries", job.retries);
        doc.insert("agents_required", job.agents_required);
        doc.insert("agents_running", job.agents_running);

        doc
    }
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
