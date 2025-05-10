use std::ops::Deref;

use rkyv::{
    Archive, Deserialize, Serialize, deserialize, rancor::Error, with::ArchiveWith, with::Inline,
};

use uuid::Uuid;

pub enum Direction {
    CommandToAgent,
    AgentToCommand,
}

#[derive(Archive, Deserialize, Serialize, PartialEq, Debug, Clone)]
pub enum Message {
    Ping,
}

impl From<&ArchivedMessage> for Message {
    fn from(archived: &ArchivedMessage) -> Self {
        match archived {
            ArchivedMessage::Ping => Message::Ping,
        }
    }
}

impl From<Message> for Vec<u8> {
    fn from(message: Message) -> Self {
        let serialized = rkyv::to_bytes::<Error>(&message).unwrap();
        serialized.to_vec()
    }
}

impl From<Vec<u8>> for Message {
    fn from(bytes: Vec<u8>) -> Self {
        let archived = rkyv::access::<ArchivedMessage, Error>(&bytes).unwrap();
        archived.into()
    }
}

pub struct Communication {
    pub id: Uuid,
    pub direction: Direction,
    pub message: Message,
    pub timestamp: i64,
}

impl Communication {
    pub fn new(direction: Direction, message: Message) -> Self {
        Self {
            id: Uuid::new_v4(),
            direction,
            message,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}
