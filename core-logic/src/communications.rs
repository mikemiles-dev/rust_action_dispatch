use std::os::unix::net::SocketAddr;

use rkyv::{Archive, Deserialize, Serialize, rancor::Error};
use uuid::Uuid;

pub enum Direction {
    CommandToAgent,
    AgentToCommand,
}

type AgentPort = u16;

#[derive(Archive, Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub enum Message {
    Ping,
    RegisterAgent(AgentPort),
}

impl From<&ArchivedMessage> for Message {
    fn from(archived: &ArchivedMessage) -> Self {
        match archived {
            ArchivedMessage::Ping => Message::Ping,
            ArchivedMessage::RegisterAgent(p) => Message::RegisterAgent(p.into()),
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
