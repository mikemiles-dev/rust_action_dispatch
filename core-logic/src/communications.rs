use uuid::Uuid;

pub enum Direction {
    CommandToAgent,
    AgentToCommand,
}

pub enum Message {
    Ping,
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
