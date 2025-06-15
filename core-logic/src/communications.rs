//! This module defines communication structures and serialization logic for messages exchanged
//! between agents and the dispatcher in a distributed system. It leverages the `rkyv` crate for
//! zero-copy serialization and deserialization, and provides asynchronous TCP write support using
//! Tokio.
//!
//! # Structures
//!
//! - `RegisterAgent`: Represents an agent registration message, containing the agent's name,
//!   hostname, and port.
//! - `DispatchJob`: Represents a job dispatch message, including job name, command, arguments, and
//!   an optional agent name.
//! - `JobComplete`: Indicates the completion of a job by an agent, including job and agent names.
//! - `Message`: An enum encapsulating all possible message types exchanged in the system.
//!
//! # Error Handling
//!
//! - `MessageError`: Enumerates possible errors during message serialization or TCP writing.
//!
//! # Serialization
//!
//! Implements conversions between `Message` and its archived form for efficient transmission over
//! the network. Provides `TryFrom` implementations for converting between `Message` and `Vec<u8>`
//! using `rkyv` serialization.
//!
//! # TCP Communication
//!
//! - `Message::tcp_write`: Asynchronously writes a serialized message to a `TcpStream`.
//!
//! # Example
//!
//! ```rust
//! use tokio::net::TcpStream;
//! use core_logic::communications::Message;
//!
//! async fn send_message(stream: &mut TcpStream, message: Message) -> Result<(), Box<dyn std::error::Error>> {
//!     message.tcp_write(stream).await?;
//!     Ok(())
//! }
//! ```
use rkyv::{Archive, Deserialize, Serialize, option::ArchivedOption, rancor::Error};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::error;

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
    pub valid_return_codes: Option<Vec<i32>>, // Optional list of valid return codes
}

#[derive(Archive, Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub enum JobOutCome {
    Failure = 0,
    Success = 1,
    Unknown,
}

impl From<&ArchivedJobOutCome> for JobOutCome {
    fn from(archived: &ArchivedJobOutCome) -> Self {
        match archived {
            ArchivedJobOutCome::Failure => JobOutCome::Failure,
            ArchivedJobOutCome::Success => JobOutCome::Success,
            ArchivedJobOutCome::Unknown => JobOutCome::Unknown,
        }
    }
}

impl From<JobOutCome> for i32 {
    fn from(outcome: JobOutCome) -> Self {
        outcome as i32
    }
}

impl From<i32> for JobOutCome {
    fn from(value: i32) -> Self {
        match value {
            0 => JobOutCome::Failure,
            1 => JobOutCome::Success,
            _ => {
                error!("Warning: Unknown JobOutCome value encountered: {}", value);
                JobOutCome::Unknown // Default to Failure for unknown values
            }
        }
    }
}

#[derive(Archive, Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct JobComplete {
    pub started_at: i64,   // Milliseconds since epoch
    pub completed_at: i64, // Milliseconds since epoch
    pub job_name: String,
    pub command: String,
    pub agent_name: String,
    pub return_code: i32,
    pub outcome: JobOutCome,
    pub output: String,
}

#[derive(Archive, Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub enum Message {
    Ping,
    RegisterAgent(RegisterAgent),
    DispatchJob(DispatchJob),
    JobComplete(JobComplete), // Job Name
}

pub enum MessageError {
    SerializationError(Error),
    WriteError(tokio::io::Error),
    AcknowledgeError(String),
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            MessageError::WriteError(e) => write!(f, "Write error: {}", e),
            MessageError::AcknowledgeError(e) => write!(f, "Acknowledge error: {}", e),
        }
    }
}

impl Message {
    pub async fn tcp_write(self, stream: &mut TcpStream) -> Result<(), MessageError> {
        let message: Vec<u8> = self.try_into().map_err(MessageError::SerializationError)?;
        stream
            .write_all(&message)
            .await
            .map_err(MessageError::WriteError)?;
        Ok(())
    }
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
                    valid_return_codes: archived
                        .valid_return_codes
                        .as_ref()
                        .map(|v| v.iter().map(|&x| x.into()).collect()),
                    agent_name,
                })
            }
            ArchivedMessage::JobComplete(archived) => {
                let job_name = archived.job_name.to_string();
                let agent_name = archived.agent_name.to_string();
                let outcome = &archived.outcome;
                let output = archived.output.to_string();
                let command = archived.command.to_string();
                Message::JobComplete(JobComplete {
                    started_at: archived.started_at.into(),
                    completed_at: archived.completed_at.into(),
                    job_name,
                    agent_name,
                    return_code: archived.return_code.into(),
                    outcome: outcome.into(),
                    command,
                    output,
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
