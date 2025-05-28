mod agent_manager;
mod command_receiver;

use tokio::spawn;
use tracing::info;

use std::error::Error;
use std::sync::Arc;

use agent_manager::AgentManager;
use command_receiver::CommandReceiver;
use core_logic::datastore::Datastore;

pub const SERVER_ADDRESS: &str = "0.0.0.0:8080";
pub const VERSION: &str = "0.1.0";

fn display_central_command_info() {
    info!("-------------------------------------------------");
    info!("\tRust Action Dispatch Central Command");
    info!("-------------------------------------------------");
    info!("\tVersion: {} Hosted at {}", VERSION, SERVER_ADDRESS);
    info!("-------------------------------------------------");
}

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
        let mut command_receiver = CommandReceiver::new(cloned_datastore).await;
        command_receiver
            .listen()
            .await
            .expect("Failed to listen for connections");
    });

    // Clone the sender for use in the agent manager
    let cloned_datastore = datastore.clone();

    // Spawn a task to connect to the server and send data
    spawn(async move {
        let agent_manager = AgentManager::new(cloned_datastore).await;
        agent_manager.start().await;
    });

    display_central_command_info();

    // Keep the main task alive
    tokio::signal::ctrl_c().await?;
    info!("Shutting down.");

    Ok(())
}
