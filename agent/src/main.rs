use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use tracing::info;

use core_logic::communications::{Communication, Direction, Message};

use std::io;

const AGNET_PORT: u16 = 8081;

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set the minimum level to display
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    ConnectionManager::try_new()
        .expect("Failed to create connection manager")
        .listen()
        .await
}

pub struct ConnectionManager {
    listener: TcpListener,
}

impl ConnectionManager {
    pub fn try_new() -> io::Result<Self> {
        let listener = std::net::TcpListener::bind(format!("0.0.0.0:{AGNET_PORT}"))?;
        listener.set_nonblocking(true)?;
        let listener = TcpListener::from_std(listener)?;

        Ok(Self { listener })
    }

    pub async fn listen(&self) -> io::Result<()> {
        info!("Listening on: {}", self.listener.local_addr()?);

        loop {}
    }
}
