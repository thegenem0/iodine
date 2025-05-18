use iodine_common::{
    error::Error,
    task::{TaskDefinition, TaskRunStatus},
};
use tracing::{error, info};
use uuid::Uuid;

use super::default::Launcher;

#[derive(Debug, Clone)]
pub(super) struct TaskToLaunch {
    task_definition: TaskDefinition,
    pipeline_run_id: Uuid,
    attempt: i32,
}

#[derive(Debug, Clone)]
pub(super) struct TaskExceedingRetries {
    task_def_id: Uuid,
    pipeline_run_id: Uuid,
    completed_attempts: i32,
    max_retries_allowed: i32,
    task_name: String,
}

#[derive(Debug)]
pub(super) struct SchedulingInfo {
    pipeline_run_id: Uuid,
    tasks_to_launch: Vec<TaskToLaunch>,
    tasks_to_fail: Vec<TaskExceedingRetries>,
}

impl Launcher {
    /// Schedules and launches tasks for the current pipeline run.
    /// ---
    /// *This is the main scheduling loop for the Launcher.*
    /// It gathers information about which tasks are ready to launch
    /// and which have exceeded retry counts.
    /// It then launches tasks that are ready to launch.
    /// If no tasks are ready to launch, it marks the pipeline run as complete.
    pub async fn schedule_and_launch_tasks(&mut self) -> Result<(), Error> {
        let sched_info_opt = self.gather_sheduling_info()?;

        let sched_info = match sched_info_opt {
            Some(info) => info,
            None => return Ok(()), // No active pipeline or task, nothing to do.
        };

        let mut handled_sched = false;
        let mut launch_empty = true;
        let mut fail_empty = true;

        if !sched_info.tasks_to_launch.is_empty() {
            if self
                .process_tasks_to_launch(sched_info.pipeline_run_id, sched_info.tasks_to_launch)
                .await?
            {
                handled_sched = true;
            }
        } else {
            launch_empty = false;
        }

        if !sched_info.tasks_to_fail.is_empty() {
            if self
                .process_tasks_exceeding_retries(sched_info.tasks_to_fail)
                .await?
            {
                handled_sched = true;
            }
        } else {
            fail_empty = false;
        }

        if handled_sched || (launch_empty && fail_empty) {
            self.check_and_finalize_pipeline_if_complete().await?;
        }

        Ok(())
    }

    /// Gathers information about which tasks are ready to launch
    /// and which have exceeded retry counts.
    pub(super) fn gather_sheduling_info(&self) -> Result<Option<SchedulingInfo>, Error> {
        let graph = self
            .execution_graph
            .as_ref()
            .ok_or_else(|| Error::Internal("Execution graph missing for scheduling".to_string()))?;

        let current_pipeline_run_id = self
            .current_pipeline_run_id
            .ok_or_else(|| Error::Internal("Pipeline run ID missing for scheduling".to_string()))?;

        let ready_task_ids = graph.get_ready_tasks(&self.task_states);
        let mut tasks_to_launch = Vec::new();
        let mut tasks_to_fail = Vec::new();

        for task_id in ready_task_ids {
            let task_def = graph.get_task_definition(task_id).ok_or_else(|| {
                Error::Internal(format!(
                    "Failed to find TaskDefinition for task ID {}",
                    task_id
                ))
            })?;

            let completed_attempts = self.task_attempts.get(&task_id).cloned().unwrap_or(0);
            let next_attempt = completed_attempts + 1;
            let max_total_attempts = task_def.max_attempts.unwrap_or(3);

            if matches!(
                self.task_states.get(&task_id),
                Some(TaskRunStatus::Pending) | Some(TaskRunStatus::Retrying) | None
            ) && !self.is_task_active(task_id)
            {
                if next_attempt <= max_total_attempts {
                    tasks_to_launch.push(TaskToLaunch {
                        task_definition: task_def.clone(),
                        pipeline_run_id: current_pipeline_run_id,
                        attempt: next_attempt,
                    });
                } else if self.task_states.get(&task_id) != Some(&TaskRunStatus::Failed) {
                    // Already exceeded max attempts and not yet marked as failed
                    tasks_to_fail.push(TaskExceedingRetries {
                        task_def_id: task_id,
                        pipeline_run_id: current_pipeline_run_id,
                        completed_attempts,
                        max_retries_allowed: max_total_attempts,
                        task_name: task_def.name.clone(),
                    });
                }
            }
        }

        Ok(Some(SchedulingInfo {
            pipeline_run_id: current_pipeline_run_id,
            tasks_to_launch,
            tasks_to_fail,
        }))
    }

    /// Launches tasks that are ready to launch.
    pub(super) async fn process_tasks_to_launch(
        &mut self,
        pipeline_run_id: Uuid,
        tasks_to_launch: Vec<TaskToLaunch>,
    ) -> Result<bool, Error> {
        if tasks_to_launch.is_empty() {
            return Ok(false);
        }

        for task_info in tasks_to_launch {
            info!(
                "Launcher [{}]: Task '{}' (ID: {}), attempt {}. Preparing to launch.",
                self.id,
                task_info.task_definition.name,
                task_info.task_definition.id,
                task_info.attempt
            );

            let task_run_id = self
                .run_def_map
                .get(&task_info.task_definition.id)
                .cloned()
                .ok_or_else(|| {
                    Error::Internal(format!(
                        "Failed to find TaskRun ID for task ID {}. This is a logic error.",
                        task_info.task_definition.id
                    ))
                })?;

            if let Err(e) = self
                .launch_task_worker(
                    &task_info.task_definition,
                    pipeline_run_id,
                    task_info.attempt,
                )
                .await
            {
                error!(
                    "Launcher [{}]: Failed to launch worker for task {} (attempt {}): {:?}. Marking as failed.",
                    self.id, task_info.task_definition.id, task_info.attempt, e
                );

                self.task_states
                    .insert(task_info.task_definition.id, TaskRunStatus::Failed);

                self.task_attempts
                    .insert(task_info.task_definition.id, task_info.attempt);

                self.state_manager
                    .update_task_run_status(
                        task_run_id,
                        TaskRunStatus::Failed,
                        None,
                        Some(format!("Launch worker failed: {}", e).into()),
                    )
                    .await?;
            } else {
                self.state_manager
                    .update_task_run_status(task_run_id, TaskRunStatus::Running, None, None)
                    .await?;
            }
        }

        Ok(true)
    }

    /// Marks tasks that have exceeded their retry count as failed.
    pub(super) async fn process_tasks_exceeding_retries(
        &mut self,
        tasks_to_fail: Vec<TaskExceedingRetries>,
    ) -> Result<bool, Error> {
        if tasks_to_fail.is_empty() {
            return Ok(false);
        }

        for fail_info in tasks_to_fail {
            let task_run_id = self
                .run_def_map
                .get(&fail_info.task_def_id)
                .cloned()
                .ok_or_else(|| {
                    Error::Internal(format!(
                        "Failed to find TaskRun ID for task ID {}. This is a logic error.",
                        fail_info.task_def_id
                    ))
                })?;

            info!(
                "Launcher [{}]: Task '{}' (ID: {}) exceeded max retries ({}) after {} attempts. Marking as failed.",
                self.id,
                fail_info.task_name,
                fail_info.task_def_id,
                fail_info.max_retries_allowed,
                fail_info.completed_attempts
            );

            self.task_states
                .insert(fail_info.task_def_id, TaskRunStatus::Failed);

            self.task_attempts
                .insert(fail_info.task_def_id, fail_info.completed_attempts);

            self.state_manager
                .update_task_run_status(
                    task_run_id,
                    TaskRunStatus::Failed,
                    None,
                    Some(
                        format!("Exceeded max retries ({})", fail_info.max_retries_allowed).into(),
                    ),
                )
                .await?;
        }

        Ok(true)
    }
}
