use iodine_common::{error::Error, resource_manager::ProvisionedWorkerStatus, task::TaskRunStatus};
use uuid::Uuid;

use super::default::Launcher;

impl Launcher {
    pub(super) async fn check_and_finalize_pipeline_if_complete(&mut self) -> Result<(), Error> {
        // FIXME(thegenem0):
        // Figure out why this is not being called.
        // Pipeline state is never being finalized,
        // even when all nodes are complete.

        if self.execution_graph.is_none() || self.current_pipeline_run_id.is_none() {
            println!("No execution graph or run ID for finalization. Skipping finalization check.");
            return Ok(());
        }

        // Pipeline can only be complete if no tasks are being actively polled and no workers are supposedly active.
        if !self.active_workers.is_empty() || !self.polling_monitor_tasks.is_empty() {
            println!(
                "Active workers or polling monitor tasks present. Skipping finalization check."
            );
            return Ok(());
        }

        let graph = self.execution_graph.as_ref().unwrap();

        if let Some(final_status) = graph.is_pipeline_complete(&self.task_states) {
            println!("Pipeline complete. Finalizing.");
            let run_id = self.current_pipeline_run_id.unwrap();
            self.finalize_pipeline(run_id, final_status, None).await?;
        }

        Ok(())
    }

    /// Helper to map ExecutionManager's ProvisionedWorkerStatus to iodine_common::task::TaskStatus
    /// # Returns
    /// - TaskStatus: The final status to report to the DAG
    /// - Option<String>: Optional message to log
    /// - bool: Whether the task is terminal (i.e. no further status updates will be expected)
    pub(super) fn map_em_status_to_task_status(
        em_status: ProvisionedWorkerStatus,
    ) -> (TaskRunStatus, Option<String>, bool) {
        match em_status {
            ProvisionedWorkerStatus::Succeeded => (
                TaskRunStatus::Succeeded,
                Some("Completed successfully (per EM)".to_string()),
                true,
            ),
            ProvisionedWorkerStatus::Failed => (
                TaskRunStatus::Failed,
                Some("Failed (per EM)".to_string()),
                true,
            ),
            ProvisionedWorkerStatus::Cancelled => (
                TaskRunStatus::Cancelled,
                Some("Cancelled (per EM)".to_string()),
                true,
            ),
            ProvisionedWorkerStatus::TimedOut => (
                TaskRunStatus::Failed,
                Some("Timed out (per EM)".to_string()),
                true,
            ),
            ProvisionedWorkerStatus::Terminated => (
                TaskRunStatus::Cancelled,
                Some("Terminated by EM (interpreted as Cancelled/Failed)".to_string()),
                true,
            ),
            ProvisionedWorkerStatus::ErrorState(e_msg) => (
                TaskRunStatus::Failed,
                Some(format!("EM ErrorState: {}", e_msg)),
                true,
            ),
            ProvisionedWorkerStatus::Running => (TaskRunStatus::Running, None, false),
            ProvisionedWorkerStatus::Initializing | ProvisionedWorkerStatus::Pending => {
                (TaskRunStatus::Queued, None, false)
            }
            ProvisionedWorkerStatus::Terminating => (
                TaskRunStatus::Running,
                Some("Terminating (per EM)".to_string()),
                false, // Still active from Launcher's POV
            ),
            ProvisionedWorkerStatus::Unknown(m_opt) => (
                TaskRunStatus::Running,
                m_opt.or(Some("Status unknown from EM".to_string())),
                false, // Assume still needs monitoring
            ),
        }
    }

    pub(super) fn is_task_active(&self, task_id: Uuid) -> bool {
        self.active_workers
            .values()
            .any(|info| info.task_id == task_id)
    }
}
