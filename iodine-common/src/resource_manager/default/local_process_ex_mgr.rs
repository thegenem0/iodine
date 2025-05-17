use std::{collections::HashMap, process::Stdio, sync::Arc, time::Duration};
use tracing::{error, info, warn};

use async_trait::async_trait;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{ChildStderr, ChildStdout},
    sync::{Mutex, oneshot},
    task::JoinHandle,
    time::{Instant, timeout},
};
use uuid::Uuid;

use crate::{
    error::Error,
    resource_manager::{
        ExecutionContext, ExecutionContextKind, ExecutionManager, ProvisionedWorkerDetails,
        ProvisionedWorkerStatus, WorkerRequest,
    },
};

#[derive(Debug)]
#[allow(dead_code)]
struct MonitoredProcess {
    pid: u32,
    status: ProvisionedWorkerStatus,
    message: Option<String>,
    start_time: Instant,
    exec_timeout: Option<Duration>,
    // Sender to signal the monitoring task to attempt cancellation.
    cancel_signal_tx: Option<oneshot::Sender<()>>,

    // Join handle for the task that awaits the child process completion.
    monitor_handle: Option<JoinHandle<()>>,
    log_buffer: Arc<Mutex<Vec<String>>>,
}

#[derive(Debug, Default)]
pub struct LocalProcessExecutionManager {
    id: Uuid,
    active_processes: Arc<Mutex<HashMap<Uuid, Arc<Mutex<MonitoredProcess>>>>>,
}

enum StdioPipes {
    Stdout(ChildStdout),
    Stderr(ChildStderr),
}

impl LocalProcessExecutionManager {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            active_processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Creates a task that captures stdout/stderr from a child process.
    /// ---
    /// The task is responsible for reading from the child process's stdout/stderr pipes
    /// and pushing the lines to a shared buffer.
    fn create_log_capture_task(
        input_pipe: StdioPipes,
        log_buffer: Arc<Mutex<Vec<String>>>,
    ) -> JoinHandle<()> {
        let stderr_log_buf = log_buffer.clone();

        tokio::spawn(async move {
            match input_pipe {
                StdioPipes::Stdout(pipe) => {
                    let mut reader = BufReader::new(pipe);
                    let mut line = String::new();
                    while let Ok(bytes_read) = reader.read_line(&mut line).await {
                        if bytes_read == 0 {
                            break;
                        }
                        log_buffer.lock().await.push(line.trim_end().to_string());
                        line.clear();
                    }
                }
                StdioPipes::Stderr(pipe) => {
                    let mut reader = BufReader::new(pipe);
                    let mut line = String::new();
                    while let Ok(bytes_read) = reader.read_line(&mut line).await {
                        if bytes_read == 0 {
                            break;
                        }
                        stderr_log_buf
                            .lock()
                            .await
                            .push(line.trim_end().to_string());
                        line.clear();
                    }
                }
            };
        })
    }
}

#[async_trait]
impl ExecutionManager for LocalProcessExecutionManager {
    fn manager_id(&self) -> Uuid {
        self.id
    }

    fn supported_resource_type(&self) -> ExecutionContextKind {
        ExecutionContextKind::LocalProcess
    }

    async fn fetch_logs(
        &self,
        details: &ProvisionedWorkerDetails,
        tail_lines: Option<usize>,
    ) -> Result<Vec<String>, Error> {
        let active_procs = self.active_processes.lock().await;
        if let Some(proc_info_arc) = active_procs.get(&details.worker_id) {
            let process_info_guard = proc_info_arc.lock().await;
            let log_buffer_guard = process_info_guard.log_buffer.lock().await;

            if let Some(n) = tail_lines {
                let log_len = log_buffer_guard.len();
                let start_idx = log_len.saturating_sub(n);
                Ok(log_buffer_guard[start_idx..log_len].to_vec())
            } else {
                Ok(log_buffer_guard.clone())
            }
        } else {
            error!("Worker {} not found for fetch_logs.", details.worker_id);

            Err(Error::Internal(format!(
                "Worker {} not found for fetch_logs.",
                details.worker_id
            )))
        }
    }

    async fn get_execution_status(
        &self,
        details: &ProvisionedWorkerDetails,
    ) -> Result<ProvisionedWorkerStatus, Error> {
        let active_procs = self.active_processes.lock().await;

        if let Some(proc_info_arc) = active_procs.get(&details.worker_id) {
            let info_guard = proc_info_arc.lock().await;

            if !matches!(
                info_guard.status,
                ProvisionedWorkerStatus::Succeeded
                    | ProvisionedWorkerStatus::Failed
                    | ProvisionedWorkerStatus::Cancelled
                    | ProvisionedWorkerStatus::TimedOut
                    | ProvisionedWorkerStatus::Terminated
                    | ProvisionedWorkerStatus::ErrorState(_)
            ) {
                if let Some(timeout) = info_guard.exec_timeout {
                    if info_guard.start_time.elapsed() > timeout {
                        // Status will be updated by the monitor task, but we can report it as Timeout here too.
                        // The monitor task is the one responsible for killing.
                        // This get_status might reflect it slightly before the monitor updates its internal state.
                        warn!(
                            "LocalProcessEM [{}]: Worker {} (PID {}) detected as timed out by get_status.",
                            self.id, details.worker_id, info_guard.pid
                        );

                        return Ok(ProvisionedWorkerStatus::TimedOut);
                    }
                }
            }
            Ok(info_guard.status.clone())
        } else {
            // If not in active_processes, it might have completed and been cleaned up,
            // or it was never properly registered. We need a way to know past statuses for cleaned up tasks.
            // For now, assume if not found, it's Unknown or Terminated if we expect cleanup.
            // This part needs robust handling of "already completed and cleaned" vs "never existed".
            // The Launcher should ideally not poll status for a worker_id it knows has terminated.

            error!(
                "LocalProcessEM [{}]: Worker {} not found in active_processes for get_status.",
                self.id, details.worker_id
            );
            Ok(ProvisionedWorkerStatus::Unknown(Some(
                "Process not actively tracked or already cleaned up.".to_string(),
            )))
        }
    }

    async fn provision_and_start_execution(
        &self,
        request: &WorkerRequest,
    ) -> Result<ProvisionedWorkerDetails, Error> {
        let local_proc_config = match &request.execution_context {
            ExecutionContext::LocalProcess(config) => config,
            _ => return Err(Error::Internal("Unsupported execution context".to_string())),
        };

        if local_proc_config.entry_point.is_empty() {
            return Err(Error::Internal("No entry point specified".to_string()));
        }

        let program = &local_proc_config.entry_point[0];
        let init_args = &local_proc_config.entry_point[1..];
        let mut command = tokio::process::Command::new(program);
        command
            .args(init_args)
            .args(&local_proc_config.args)
            .envs(&local_proc_config.env_vars)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child_proc = command
            .spawn()
            .map_err(|err| Error::Internal(format!("Failed to spawn local process: {}", err)))?;

        let pid = child_proc
            .id()
            .ok_or_else(|| Error::Internal("Failed to get local process ID".to_string()))?;

        let worker_id = request.worker_id;
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        let shared_log_buffer = Arc::new(Mutex::new(Vec::new()));

        let child_stdout = child_proc
            .stdout
            .take()
            .ok_or_else(|| Error::Internal("Failed to get local process stdout".to_string()))?;

        let child_stderr = child_proc
            .stderr
            .take()
            .ok_or_else(|| Error::Internal("Failed to get local process stderr".to_string()))?;

        let process_info = Arc::new(Mutex::new(MonitoredProcess {
            pid,
            status: ProvisionedWorkerStatus::Initializing,
            message: None,
            start_time: Instant::now(),
            exec_timeout: local_proc_config.exec_timeout,
            cancel_signal_tx: Some(cancel_tx),
            monitor_handle: None,
            log_buffer: Arc::clone(&shared_log_buffer),
        }));

        let proc_info_for_monitor = process_info.clone();
        let exec_timout_for_monitor = local_proc_config.exec_timeout;

        let monitor_handle = tokio::spawn(async move {
            let stdout_task = Self::create_log_capture_task(
                StdioPipes::Stdout(child_stdout),
                shared_log_buffer.clone(),
            );

            let stderr_task = Self::create_log_capture_task(
                StdioPipes::Stderr(child_stderr),
                shared_log_buffer.clone(),
            );

            let status_future = child_proc.wait();
            let mut timeout_fired = false;

            let final_status_res = tokio::select! {
                biased;

                _ = cancel_rx => {
                    if let Err(e) = child_proc.kill().await {
                        error!(
                            "[Monitor {} PID {}] Failed to kill process after cancel signal: {}",
                            worker_id,
                            pid,
                            e,
                        );
                    }

                    Ok((ProvisionedWorkerStatus::Cancelled, Some("Cancelled".to_string()))) as Result<_, Error>
                },

                exit_result = status_future => {
                    match exit_result {
                        Ok(status) => {
                            let message = format!("Process exited with status: {}", status);
                            info!("[Monitor {} PID {}] {}", worker_id, pid, message);

                            if status.success() {
                                Ok((ProvisionedWorkerStatus::Succeeded, Some(message)))
                            } else {
                                Ok((ProvisionedWorkerStatus::Failed, Some(message)))
                            }
                        }
                        Err(e) => {
                            let message = format!("Error waiting for process exit: {}", e);
                            error!("[Monitor {} PID {}] {}", worker_id, pid, message);
                            Ok((ProvisionedWorkerStatus::ErrorState(message.clone()), Some(message)))
                        }
                    }
                },

                _ = async {
                        if let Some(timeout_dur) = exec_timout_for_monitor {
                            tokio::time::sleep(timeout_dur).await;
                            warn!(
                                "[Monitor {} PID {}] Execution timeout of {:?} reached.",
                                worker_id,
                                pid,
                                timeout_dur,
                            );
                            timeout_fired = true;
                        } else {
                            std::future::pending::<()>().await;
                        }
                    }, if exec_timout_for_monitor.is_some() && !timeout_fired => {
                        info!("[Monitor {} PID {}] Timeout triggered, attempting to kill process.", worker_id, pid);
                        if let Ok(status) = child_proc.wait().await {
                            info!("[Monitor {} PID {}] Process exited with status: {:?}", worker_id, pid, status);

                            if let Err(e) = child_proc.kill().await {
                                error!("[Monitor {} PID {}] Failed to kill process after timeout: {}", worker_id, pid, e);
                            }
                        } else {
                            info!("[Monitor {} PID {}] Process already exited", worker_id, pid);
                        }

                        Ok((
                            ProvisionedWorkerStatus::TimedOut,
                            Some(format!("Execution timed out after {:?}", exec_timout_for_monitor)),
                        ))
                    },
            };

            // Wait for log capturing tasks to finish (they will end when pipes close after process exit)
            // Small timeout for these log tasks to avoid hanging indefinitely if a pipe issue occurs.
            let log_collection_timeout = Duration::from_secs(2);

            if timeout(log_collection_timeout, stdout_task).await.is_err() {
                error!(
                    "[Monitor {} PID {}] Timeout waiting for stdout log task to complete.",
                    worker_id, pid
                );
            }

            if timeout(log_collection_timeout, stderr_task).await.is_err() {
                error!(
                    "[Monitor {} PID {}] Timeout waiting for stderr log task to complete.",
                    worker_id, pid,
                );
            }

            let mut info_guard = proc_info_for_monitor.lock().await;
            match final_status_res {
                Ok((status, message)) => {
                    info_guard.status = status;
                    info_guard.message = message;
                }
                Err(_internal_in_select_err) => {
                    info_guard.status =
                        ProvisionedWorkerStatus::ErrorState("Monitor task logic error".to_string());
                }
            }

            info_guard.cancel_signal_tx = None;
            info!(
                "[Monitor {} PID {}] Finished with status {:?}.",
                worker_id, pid, info_guard.status,
            );
        });

        process_info.lock().await.monitor_handle = Some(monitor_handle);

        self.active_processes
            .lock()
            .await
            .insert(worker_id, process_info);

        Ok(ProvisionedWorkerDetails {
            worker_id,
            platform_id: format!("local_process_{}", pid),
            manager_id: self.id,
            resource_type_provisioned: ExecutionContextKind::LocalProcess,
        })
    }

    async fn cancel_execution(&self, details: &ProvisionedWorkerDetails) -> Result<(), Error> {
        let active_procs = self.active_processes.lock().await;

        if let Some(proc_info_arc) = active_procs.get(&details.worker_id) {
            let mut info_guard = proc_info_arc.lock().await;

            if let Some(cancel_tx) = info_guard.cancel_signal_tx.take() {
                if cancel_tx.send(()).is_ok() {
                    info!(
                        "LocalProcessEM [{}]: Cancellation signal sent to monitor for worker {}.",
                        self.id, details.worker_id,
                    );

                    // The monitor task will handle actual killing and status update
                    info_guard.status = ProvisionedWorkerStatus::Terminating; // Tentative status
                    Ok(())
                } else {
                    warn!(
                        "LocalProcessEM [{}]: Failed to send cancel signal to monitor for worker {}. Current status: {:?}.",
                        self.id, details.worker_id, info_guard.status,
                    );

                    // If monitor already exited, process might be done or stuck.
                    // If it's still running and monitor exited, this is an issue.
                    // We might need to directly try to kill the PID if known and not already terminal.
                    if !matches!(
                        info_guard.status,
                        ProvisionedWorkerStatus::Succeeded
                            | ProvisionedWorkerStatus::Failed
                            | ProvisionedWorkerStatus::Cancelled
                            | ProvisionedWorkerStatus::Terminated
                            | ProvisionedWorkerStatus::ErrorState(_)
                    ) {
                        info!(
                            "LocalProcessEM [{}]: Attempting direct kill for PID {:?} as fallback.",
                            self.id, info_guard.pid,
                        );

                        #[cfg(unix)]
                        {
                            if let Err(e) = std::process::Command::new("pkill")
                                .arg(format!("{}", info_guard.pid))
                                .output()
                            {
                                error!(
                                    "LocalProcessEM [{}]: Failed to send cancel signal to monitor for worker {}. Current status: {:?}. Error: {:?}.",
                                    self.id, details.worker_id, info_guard.status, e
                                );

                                return Err(Error::Internal(format!(
                                    "Failed to send cancel signal for worker {}",
                                    details.worker_id
                                )));
                            } else {
                                info!(
                                    "LocalProcessEM [{}]: Cancellation signal sent to monitor for worker {}.",
                                    self.id, details.worker_id,
                                );

                                // The monitor task will handle actual killing and status update
                                info_guard.status = ProvisionedWorkerStatus::Terminating; // Tentative status
                            }
                        }
                    }

                    Ok(())
                }
            } else {
                // No cancel_tx means task is likely already terminating or completed

                info!(
                    "LocalProcessEM [{}]: Worker {} already terminating or completed, no cancel signal sent. Current status: {:?}.",
                    self.id, details.worker_id, info_guard.status,
                );

                Ok(()) // Or error if status isn't terminal?
            }
        } else {
            Err(Error::Internal(format!(
                // Or NotFound
                "Worker {} not found for cancellation.",
                details.worker_id
            )))
        }
    }

    async fn teardown_worker(&self, details: &ProvisionedWorkerDetails) -> Result<(), Error> {
        self.cancel_execution(details).await?;

        let mut active_procs = self.active_processes.lock().await;
        if let Some(proc_info_arc) = active_procs.remove(&details.worker_id) {
            let _handle = {
                let mut info_guard = proc_info_arc.lock().await;
                if let Some(handle) = info_guard.monitor_handle.take() {
                    handle.abort()
                }

                // Takes the handle to await it.
                // This logic is tricky: monitor_task_handle is set after spawn.
                // For now, we assume monitor task cleans itself up when child process ends or is cancelled.
                // Direct await here might deadlock if monitor task is waiting for this lock.
                // A better approach is for monitor task to remove itself from active_procs map upon its own clean exit.
                // For now, just removing from the map implies it's "torn down" from EM's perspective.
                // The actual process kill is handled by cancel_execution via the monitor.
            };

            // It's better if the monitor task removes the entry from active_processes upon its own completion.
            // For now, this manual removal signifies the EM is done with it.

            info!(
                "LocalProcessEM [{}]: Worker {} removed from active tracking.",
                self.id, details.worker_id,
            );

            Ok(())
        } else {
            // Idempotent: if not found, consider it torn down.
            Err(Error::Internal(format!(
                "Worker {} not found for teardown.",
                details.worker_id
            )))
        }
    }
}
