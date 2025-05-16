mod agent_manager;
mod command_receiver;

use mongodb::Client;
use std::sync::Arc;
use tokio::spawn;
use tracing::info;

use std::error::Error;

use agent_manager::AgentManager;
use command_receiver::CommandReceiver;
use core_logic::datastore::Datastore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up tracing subscriber for logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO) // Set the minimum level to display
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    // Initialize the datastore
    let datastore = Arc::new(
        Datastore::try_new()
            .await
            .expect("Failed to create datastore"),
    );

    let cloned_datastore = datastore.clone();

    spawn(async move {
        let mut command_receiver = CommandReceiver::try_new(cloned_datastore)
            .await
            .expect("Failed to create connection manager");
        command_receiver
            .listen()
            .await
            .expect("Failed to listen for connections");
    });

    // Clone the sender for use in the agent manager
    let cloned_datastore = datastore.clone();

    // Spawn a task to connect to the server and send data
    spawn(async move {
        let mut agent_manager = AgentManager::new().await;
        agent_manager.start().await;
    });

    info!("Central Command started and listening for connections...");

    // Keep the main task alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down.");

    Ok(())
}
