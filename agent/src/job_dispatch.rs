use std::sync::Arc;
use tokio::process::Command;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Sender};
use tokio::time::{Duration, sleep};

use tracing::{error, info};

use crate::{CentralCommandWriter, get_agent_name, get_agent_port};
use core_logic::communications::{DispatchJob, JobComplete, Message};

pub struct JobDispatcher {
    sender: Sender<String>,
}

impl JobDispatcher {
    pub fn new(central_command_writer: Arc<Mutex<CentralCommandWriter>>) -> Self {
        let (sender, mut receiver) = mpsc::channel::<String>(100);

        let cloned_central_command_writer = central_command_writer.clone();

        spawn(async move {
            while let Some(job_name) = receiver.recv().await {
                //info!("Received job: {}", job_name);
                // Here you would handle the job, e.g., by sending it to the central command
                let mut writer = cloned_central_command_writer.lock().await;
                let message = Message::JobComplete(JobComplete {
                    job_name: job_name.clone(),
                    agent_name: get_agent_name(), // Replace with actual agent name if needed
                });
                writer.write(message).await;
                //drop(cloned_central_command_writer)
            }
        });

        JobDispatcher { sender }
    }

    // Todo make real command runner
    pub async fn spawn(&mut self, job: DispatchJob) {
        let sender = self.sender.clone();
        spawn(async move {
            let job_name = job.job_name.clone();
            let command = job.command.clone();
            let args = job.args.clone();
            // Here you would run the job, e.g., by executing a command
            info!("Spawning job: {} with command: {}", job_name, command);

            let mut command = Command::new(command);

            command.args(args.split_whitespace());

            let output = match command.output().await {
                Ok(output) => output,
                Err(e) => {
                    error!("Failed to execute command: {}", e);
                    return;
                }
            };

            // 4. Process the output.
            // if output.status.success() {
            //     if let Err(e) = io::stdout().write_all(&output.stdout) {
            //         error!("Failed to write to stdout: {}", e);
            //     }
            //     error!("Command successful.");
            // } else {
            //     if let Err(e) = io::stderr().write_all(&output.stderr) {
            //         error!("Failed to write to stderr: {}", e);
            //     }
            //     error!("Command failed with status: {}", output.status);
            // }

            // Get the numerical return code (if available)
            if let Some(code) = output.status.code() {
                println!("Return Code: {}", code);
            } else {
                println!("Command terminated by signal (no return code).");
            }

            info!("Output is: {:?}", output);

            sleep(Duration::from_secs(5)).await;
            info!("Job {} completed", job_name);

            if let Err(e) = sender.send(job_name).await {
                error!("Failed to send job name: {}", e);
            }
        });
    }
}
