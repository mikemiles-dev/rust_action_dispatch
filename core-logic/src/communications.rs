use std::net::SocketAddr;

use rkyv::{Archive, Deserialize, Serialize, option::ArchivedOption, rancor::Error};
use uuid::Uuid;

pub enum Direction {
    CommandToAgent,
    AgentToCommand,
}

#[derive(Archive, Deserialize, Serialize, Hash, PartialEq, Eq, Debug, Clone)]
pub struct RegisterAgent {
    pub name: String,
    pub hostname: String,
    pub port: u16,
}

#[derive(Archive, Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct DispatchJob {
    pub job_name: String,
    pub command: String,
    pub args: String,
    pub agent_name: Option<String>,
}

#[derive(Archive, Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct JobComplete {
    pub job_name: String,
    pub agent_name: String,
}

#[derive(Archive, Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub enum Message {
    Ping,
    RegisterAgent(RegisterAgent),
    DispatchJob(DispatchJob),
    JobComplete(JobComplete), // Job Name
}

impl From<&ArchivedMessage> for Message {
    fn from(archived: &ArchivedMessage) -> Self {
        match archived {
            ArchivedMessage::Ping => Message::Ping,
            ArchivedMessage::RegisterAgent(archived) => {
                let name = archived.name.to_string();
                let hostname = archived.hostname.to_string();
                let port = archived.port.into();
                Message::RegisterAgent(RegisterAgent {
                    name,
                    hostname,
                    port,
                })
            }
            ArchivedMessage::DispatchJob(archived) => {
                let job_name = archived.job_name.to_string();
                let job_command = archived.command.to_string();
                let job_args = archived.args.to_string();
                let agent_name = match &archived.agent_name {
                    ArchivedOption::None => None,
                    ArchivedOption::Some(name) => Some(name.to_string()),
                };
                Message::DispatchJob(DispatchJob {
                    job_name: job_name.to_string(),
                    command: job_command,
                    args: job_args.to_string(),
                    agent_name,
                })
            }
            ArchivedMessage::JobComplete(archived) => {
                let job_name = archived.job_name.to_string();
                let agent_name = archived.agent_name.to_string();
                Message::JobComplete(JobComplete {
                    job_name,
                    agent_name,
                })
            }
        }
    }
}

impl TryFrom<Message> for Vec<u8> {
    type Error = Error;

    fn try_from(message: Message) -> Result<Vec<u8>, Error> {
        let serialized = rkyv::to_bytes::<Error>(&message)?;
        Ok(serialized.to_vec())
    }
}

impl TryFrom<Vec<u8>> for Message {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Error> {
        let archived = rkyv::access::<ArchivedMessage, Error>(&bytes)?;
        Ok(archived.into())
    }
}

pub struct Communication {
    pub id: Uuid,
    pub direction: Direction,
    pub agent: SocketAddr,
    pub message: Message,
    pub timestamp: i64,
}

impl Communication {
    pub fn new(direction: Direction, agent: SocketAddr, message: Message) -> Self {
        Self {
            id: Uuid::new_v4(),
            direction,
            agent,
            message,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}
