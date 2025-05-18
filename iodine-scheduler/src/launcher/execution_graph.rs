use std::collections::{HashMap, HashSet};

use iodine_common::{
    error::Error,
    pipeline::{PipelineDefinition, PipelineRunStatus},
    task::{TaskDefinition, TaskRunStatus},
};
use petgraph::{
    Direction::{Incoming, Outgoing},
    algo::is_cyclic_directed,
    graph::{DiGraph, NodeIndex},
};
use tracing::warn;
use uuid::Uuid;

pub struct PipelineExecutionGraph {
    /// The directed graph where nodes are task definitions and edges represent dependencies.
    /// An edge from Task A to Task B means Task A must complete before Task B can start.
    graph: DiGraph<TaskDefinition, ()>,

    /// Mapping from a task's Uuid to its NodeIndex in the petgraph.
    task_id_to_node_idx: HashMap<Uuid, NodeIndex>,

    /// Mapping from a NodeIndex to its task's Uuid.
    node_idx_to_task_id: HashMap<NodeIndex, Uuid>,

    /// Set of task IDs that are sink nodes (no outgoing dependencies).
    sink_nodes: HashSet<Uuid>,

    /// Set of task IDs that are source nodes (no incoming dependencies).
    source_nodes: HashSet<Uuid>,
}

impl PipelineExecutionGraph {
    /// Creates a new `PipelineExecutionGraph` from a `PipelineDefinition`.
    /// This involves adding all tasks as nodes and then adding edges based on the
    /// `depends_on` field of each task.
    /// It also validates the graph (e.g., for cycles).
    pub fn new(pipeline_def: &PipelineDefinition) -> Result<Self, Error> {
        let mut graph = DiGraph::new();
        let mut task_id_to_node_idx = HashMap::new();
        let mut node_idx_to_task_id = HashMap::new();

        if pipeline_def.task_definitions.is_empty() {
            warn!(
                "Warning: Creating an execution graph for an empty pipeline definition (ID: {}).",
                pipeline_def.info.id
            );

            return Ok(Self {
                graph,
                task_id_to_node_idx,
                node_idx_to_task_id,
                sink_nodes: HashSet::new(),
                source_nodes: HashSet::new(),
            });
        }

        for task_def in &pipeline_def.task_definitions {
            if task_id_to_node_idx.contains_key(&task_def.id) {
                return Err(Error::Conflict(format!(
                    "Duplicate task ID {} found in pipeline definition {}.",
                    task_def.id, pipeline_def.info.id
                )));
            }

            let node_idx = graph.add_node(task_def.clone());
            task_id_to_node_idx.insert(task_def.id, node_idx);
            node_idx_to_task_id.insert(node_idx, task_def.id);

            for dep_task_id in &task_def.depends_on {
                let from_node_idx = task_id_to_node_idx.get(dep_task_id).ok_or_else(|| {
                    Error::Internal(format!(
                        "Failed to find task ID {} in task_id_to_node_idx for pipeline definition {}.",
                        dep_task_id, pipeline_def.info.id
                    ))
                })?;

                graph.add_edge(*from_node_idx, node_idx, ());
            }
        }

        if is_cyclic_directed(&graph) {
            return Err(Error::Conflict(format!(
                "Cyclic dependency graph detected for pipeline definition {}.",
                pipeline_def.info.id
            )));
        }

        let mut source_nodes = HashSet::new();
        let mut sink_nodes = HashSet::new();

        for (node_idx, task_def_id) in &node_idx_to_task_id {
            if graph.neighbors_directed(*node_idx, Incoming).count() == 0 {
                source_nodes.insert(*task_def_id);
            }

            if graph.neighbors_directed(*node_idx, Outgoing).count() == 0 {
                sink_nodes.insert(*task_def_id);
            }
        }

        Ok(Self {
            graph,
            task_id_to_node_idx,
            node_idx_to_task_id,
            sink_nodes,
            source_nodes,
        })
    }

    /// Retrieves a reference to a task definition by its ID.
    pub fn get_task_definition(&self, task_def_id: Uuid) -> Option<&TaskDefinition> {
        self.task_id_to_node_idx
            .get(&task_def_id)
            .and_then(|node_idx| self.graph.node_weight(*node_idx))
    }

    /// Gets all task IDs present in the graph.
    pub fn get_all_task_ids(&self) -> Vec<Uuid> {
        self.node_idx_to_task_id.values().cloned().collect()
    }

    /// Gets the IDs of tasks that have no incoming dependencies (source nodes).
    /// These are the tasks that can potentially start first.
    pub fn get_source_task_ids(&self) -> &HashSet<Uuid> {
        &self.source_nodes
    }

    /// Identifies tasks whose direct dependencies have all successfully completed.
    ///
    /// # Arguments
    /// * `current_task_states`: A map of task IDs to their current `TaskStatus`.
    ///
    /// # Returns
    /// A vector of `Uuid`s for tasks that are now ready to be scheduled.
    pub fn get_ready_tasks(&self, current_task_states: &HashMap<Uuid, TaskRunStatus>) -> Vec<Uuid> {
        let mut ready_tasks = Vec::new();

        for (node_idx, task_def_id) in &self.node_idx_to_task_id {
            match current_task_states.get(task_def_id) {
                Some(TaskRunStatus::Pending) => {
                    let mut dependencies_met = true;

                    for predecessor_node_idx in self.graph.neighbors_directed(*node_idx, Incoming) {
                        let dep_task_id = self
                            .node_idx_to_task_id
                            .get(&predecessor_node_idx)
                            .expect("Failed to find predecessor node index in node_idx_to_task_id");

                        match current_task_states.get(dep_task_id) {
                            Some(TaskRunStatus::Succeeded) => { /* Dependency met, continue */ }
                            _ => {
                                dependencies_met = false;
                                break;
                            }
                        }
                    }

                    if dependencies_met {
                        ready_tasks.push(*task_def_id);
                    }
                }
                _ => {
                    /* Task is already running, completed, failed, etc.
                     * Not "ready" for a new launch in this cycle. */
                }
            }
        }

        ready_tasks
    }

    /// Checks if the entire pipeline has completed based on the states of its tasks.
    ///
    /// # Arguments
    /// * `current_task_states`: A map of task IDs to their current `TaskStatus`.
    ///
    /// # Returns
    /// * `Some(PipelineRunStatus::CompletedSuccess)` if all tasks finished successfully.
    /// * `Some(PipelineRunStatus::CompletedFailure)` if any task failed (and isn't retrying/pending retry)
    ///   or was cancelled, making overall success impossible.
    /// * `None` if the pipeline is still running (some tasks are not yet in a terminal state).
    pub fn is_pipeline_complete(
        &self,
        current_task_states: &HashMap<Uuid, TaskRunStatus>,
    ) -> Option<PipelineRunStatus> {
        if self.graph.node_count() == 0 {
            return Some(PipelineRunStatus::Succeeded); // Empty pipeline is always successful
        }

        let mut all_tasks_accounted_for = true;
        let mut any_task_failed = false;

        for task_def_id in self.node_idx_to_task_id.values() {
            match current_task_states.get(task_def_id) {
                Some(TaskRunStatus::Succeeded) => continue,
                Some(TaskRunStatus::Failed)
                | Some(TaskRunStatus::Cancelled)
                | Some(TaskRunStatus::Skipped) => {
                    any_task_failed = true;
                }
                Some(TaskRunStatus::Pending)
                | Some(TaskRunStatus::Running)
                | Some(TaskRunStatus::Retrying)
                | Some(TaskRunStatus::Queued)
                | Some(TaskRunStatus::Cancelling) => {
                    all_tasks_accounted_for = false;
                    break; // Pipeline is definitely not complete yet
                }
                None => {
                    // Task state missing, means it hasn't even started or error in state tracking
                    all_tasks_accounted_for = false;
                    break; // Pipeline is definitely not complete yet
                }
            }
        }

        if all_tasks_accounted_for {
            if any_task_failed {
                Some(PipelineRunStatus::Failed)
            } else {
                Some(PipelineRunStatus::Succeeded)
            }
        } else {
            None // Pipeline still had active or pending tasks
        }
    }
}
