use std::sync::Arc;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Sender};
use tokio::time::{Duration, sleep};

use tracing::{error, info};

use crate::CentralCommandWriter;
use core_logic::communications::Message;

pub struct JobDispatcher {
    central_command_writer: Arc<Mutex<CentralCommandWriter>>,
    sender: Sender<String>,
}

impl JobDispatcher {
    pub fn new(central_command_writer: Arc<Mutex<CentralCommandWriter>>) -> Self {
        let (sender, mut receiver) = mpsc::channel::<String>(100);

        let cloned_central_command_writer = central_command_writer.clone();

        spawn(async move {
            while let Some(job_name) = receiver.recv().await {
                info!("Received job: {}", job_name);
                // Here you would handle the job, e.g., by sending it to the central command
                let mut writer = cloned_central_command_writer.lock().await;
                let message = Message::JobComplete;
                writer.write(message).await;
                //drop(cloned_central_command_writer)
            }
        });

        JobDispatcher {
            central_command_writer,
            sender,
        }
    }

    // Todo make real command runner
    pub async fn spawn(&mut self, job_name: String) {
        let sender = self.sender.clone();
        spawn(async move {
            info!("Spawning job: {}", job_name);
            // Simulate job processing
            sleep(Duration::from_secs(5)).await;
            info!("Job {} completed", job_name);
            sender.send(job_name).await.unwrap();
        });
    }
}
