mod agent;
mod connection_manager;

use tokio::spawn;
use tracing::info;

use agent::AgentManager;
use connection_manager::ConnectionManager;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up tracing subscriber for logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set the minimum level to display
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    spawn(async move {
        let mut connection_manager = ConnectionManager::try_new()
            .await
            .expect("Failed to create connection manager");
        connection_manager
            .listen()
            .await
            .expect("Failed to listen for connections");
    });

    // Spawn a task to connect to the server and send data
    spawn(async move {
        AgentManager::default().start().await;
    });

    info!("Central Command started and listening for connections...");

    // Keep the main task alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down.");

    Ok(())
}
