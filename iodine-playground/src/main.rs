use std::{collections::HashMap, sync::Arc, time::Duration}; // Added Arc

use iodine_common::resource_manager::{
    ExecutionContext, ExecutionManager, LocalProcessExecutionContext, LocalProcessExecutionManager,
    ProvisionedWorkerDetails, ProvisionedWorkerStatus, WorkerRequest,
};
use tracing::warn;
use uuid::Uuid;

// --- Helper function to create WorkerRequests easily ---
fn create_local_worker_request(
    worker_id: Uuid,
    command_script: &str,
    timeout_seconds: Option<u64>,
) -> WorkerRequest {
    WorkerRequest {
        pipeline_id: Uuid::new_v4(), // Dummy for test
        run_id: Uuid::new_v4(),      // Dummy for test
        worker_id,                   // Use the provided worker_id
        execution_context: ExecutionContext::LocalProcess(LocalProcessExecutionContext {
            // Struct variant construction
            entry_point: vec!["sh".to_string()], // Use shell for sleep, echo etc.
            args: vec!["-c".to_string(), command_script.to_string()],
            env_vars: HashMap::new(),
            exec_timeout: timeout_seconds.map(Duration::from_secs),
        }),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .init();

    let em = Arc::new(LocalProcessExecutionManager::new());

    // --- Define Worker Requests ---
    let worker_id_1 = Uuid::new_v4();
    let request1 = create_local_worker_request(
        worker_id_1,
        "echo 'Worker 1: Quick success!' && exit 0",
        Some(5),
    );

    let worker_id_2 = Uuid::new_v4();
    let request2 = create_local_worker_request(
        worker_id_2,
        "echo 'Worker 2: Starting long sleep...' && sleep 10 && echo 'Worker 2: Long sleep finished (should not see this)'",
        Some(3),
    );

    let worker_id_3 = Uuid::new_v4();
    let request3 = create_local_worker_request(
        worker_id_3,
        "echo 'Worker 3: Intentional failure!' && exit 123",
        Some(5),
    );

    let worker_id_4 = Uuid::new_v4();
    let request4 = create_local_worker_request(
        worker_id_4,
        "echo 'Worker 4: Success (no timeout configured for worker)' && exit 0",
        None,
    );

    let requests = vec![request1, request2, request3, request4];
    let mut worker_details_list: Vec<ProvisionedWorkerDetails> = Vec::new();
    let mut worker_final_statuses: HashMap<Uuid, ProvisionedWorkerStatus> = HashMap::new();

    for req in &requests {
        match em.provision_and_start_execution(req).await {
            Ok(details) => {
                assert_eq!(
                    req.worker_id, details.worker_id,
                    "Mismatch in worker ID tracking!"
                );
                worker_details_list.push(details);
            }
            Err(_e) => {
                worker_final_statuses.insert(req.worker_id, ProvisionedWorkerStatus::Failed);
            }
        }
    }

    let max_polling_time = Duration::from_secs(20); // Max time to wait for all workers
    let polling_start_time = std::time::Instant::now();

    while worker_final_statuses.len() < requests.len()
        && polling_start_time.elapsed() < max_polling_time
    {
        for details in &worker_details_list {
            // Only poll if we haven't recorded a final status for this worker yet
            if worker_final_statuses.contains_key(&details.worker_id) {
                continue;
            }

            match em.get_execution_status(details).await {
                Ok(status) => {
                    println!(
                        "Status for worker {}: ({}): {:?}",
                        details.worker_id, details.platform_id, status
                    );
                    match status {
                        ProvisionedWorkerStatus::Succeeded |
                        ProvisionedWorkerStatus::Failed |
                        ProvisionedWorkerStatus::Cancelled | // Assuming cancel is terminal
                        ProvisionedWorkerStatus::TimedOut |   // Assuming timeout is terminal
                        ProvisionedWorkerStatus::Terminated | // If explicitly torn down
                        ProvisionedWorkerStatus::ErrorState(_) => {
                            worker_final_statuses.insert(details.worker_id, status);
                        }
                        _ => { /* Still pending, initializing, or running */ }
                    }
                }
                Err(e) => {
                    worker_final_statuses.insert(
                        details.worker_id,
                        ProvisionedWorkerStatus::ErrorState(e.to_string()),
                    );
                }
            }
        }
        if worker_final_statuses.len() < requests.len() {
            tokio::time::sleep(Duration::from_secs(1)).await; // Polling interval
        }
    }

    if polling_start_time.elapsed() >= max_polling_time {
        warn!("\n--- Max polling time reached ---");
    }

    for req in &requests {
        match worker_final_statuses.get(&req.worker_id) {
            Some(status) => println!(
                "Task (Worker ID {}): Final Status: {:?}",
                req.worker_id, status
            ),
            None => println!(
                "Task (Worker ID {}): Final Status: Unknown (did not complete or status fetch failed).",
                req.worker_id
            ),
        }
    }

    for details in worker_details_list {
        println!(
            "Tearing down worker {} (Platform ID: {}) for task",
            details.worker_id, details.platform_id,
        );
        if let Err(e) = em.teardown_worker(&details).await {
            eprintln!("Failed to teardown worker {}: {:?}", details.worker_id, e);
        } else {
            println!("Teardown complete for worker {}", details.worker_id);
        }
    }
}
