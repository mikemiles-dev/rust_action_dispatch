/// `JobDispatcher` is responsible for managing and dispatching jobs to be executed asynchronously.
/// It communicates job completion back to a central command writer.
///
/// # Fields
/// - `sender`: An asynchronous channel sender used to queue job names for completion notification.
///
/// # Example
/// ```rust
/// let dispatcher = JobDispatcher::new(central_command_writer);
/// dispatcher.spawn(job).await;
/// ```
///
/// # Usage
/// - Use `JobDispatcher::new` to create a new dispatcher, passing an `Arc<Mutex<CentralCommandWriter>>`.
/// - Call `spawn` with a `DispatchJob` to execute a job asynchronously.
/// - Upon job completion, a `JobComplete` message is sent to the central command.
///
/// # Notes
/// - The actual command execution is performed using `tokio::process::Command`.
/// - Job completion is notified via an mpsc channel and handled in a background task.
/// - Logging is performed using the `tracing` crate.
use bson::DateTime;
use std::sync::Arc;
use tokio::process::Command;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Sender};

use tracing::{error, info};

use crate::{CentralCommandWriter, get_agent_name};
use core_logic::communications::{DispatchJob, JobComplete, JobOutCome, Message};

pub struct JobDispatcher {
    sender: Sender<JobComplete>,
}

impl JobDispatcher {
    pub fn new(central_command_writer: Arc<Mutex<CentralCommandWriter>>) -> Self {
        let (sender, mut receiver) = mpsc::channel::<JobComplete>(100);

        spawn(async move {
            while let Some(job_info) = receiver.recv().await {
                //info!("Received job: {}", job_name);
                // Here you would handle the job, e.g., by sending it to the central command
                let message = Message::JobComplete(JobComplete {
                    started_at: job_info.started_at,
                    completed_at: job_info.completed_at,
                    job_name: job_info.job_name.clone(),
                    agent_name: get_agent_name(),
                    outcome: job_info.outcome,
                    return_code: job_info.return_code,
                    data: [0u8; 1000000].to_vec(), // Placeholder for job data
                });
                let mut writer = central_command_writer.lock().await;
                writer.write(message).await;
                drop(writer); // Explicitly drop the lock to release it
            }
        });

        JobDispatcher { sender }
    }

    // Todo make real command runner
    pub async fn spawn(&mut self, job: DispatchJob) {
        let sender = self.sender.clone();
        spawn(async move {
            let job_name = job.job_name.clone();
            let command_name = job.command.clone();
            let args = job.args.clone();
            let valid_return_codes = job.valid_return_codes.clone();
            // Here you would run the job, e.g., by executing a command
            info!("Spawning job: {} with command: {}", job_name, command_name);

            let start_time = DateTime::now();

            let mut command = Command::new(command_name.clone());

            command.args(args.split_whitespace());

            let output = match command.output().await {
                Ok(output) => Some(output),
                Err(e) => {
                    error!("Failed to execute command: {}", e);
                    None
                }
            };

            let return_code = output.as_ref().and_then(|o| o.status.code()).unwrap_or(-1);

            let outcome = match valid_return_codes {
                Some(valid_codes) if valid_codes.contains(&return_code) => JobOutCome::Success,
                _ => JobOutCome::Failure,
            };

            let _output = info!("Output is: {:?}", output);
            info!("Job {} completed", job_name);

            let end_time = DateTime::now();

            let job_complete = JobComplete {
                started_at: start_time.timestamp_millis(),
                completed_at: end_time.timestamp_millis(),
                job_name: job_name.clone(),
                agent_name: get_agent_name(),
                outcome,
                return_code,
                data: vec![],
            };

            if let Err(e) = sender.send(job_complete).await {
                error!("Failed to send job name: {}", e);
            }
        });
    }
}
