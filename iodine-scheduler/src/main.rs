use std::{env, sync::Arc, time::Duration};

use coordinator::{CoordinatorConfig, SupervisorCommand, default::Coordinator};
use discovery::RegistryDiscoveryService;
use iodine_api_graphql::server::GraphQLServer;
use iodine_common::{
    command::{CommandRouter, DiscoveryCommand},
    coordinator::CoordinatorCommand,
    resource_manager::LocalProcessExecutionManager,
    state::DatabaseTrait,
};
use iodine_persistence_pg::db::PostgresStateDb;
use launcher::{LauncherStatus, exec_mgr_registry::ExecutionManagerRegistry};
use tokio::{sync::oneshot, task::JoinHandle};

#[allow(dead_code)]
mod coordinator;
mod discovery;
#[allow(dead_code)]
mod launcher;

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let state_db = PostgresStateDb::new(&database_url)
        .await
        .expect("Failed to connect to database");

    let db_arc: Arc<dyn DatabaseTrait> = Arc::new(state_db);

    let cmd_router = Arc::new(CommandRouter::new(None, None));

    // register command handlers
    cmd_router
        .register_singleton_handler::<CoordinatorCommand>()
        .await
        .expect("Failed to register CoordinatorCommand handler");

    cmd_router
        .register_singleton_handler::<LauncherStatus>()
        .await
        .expect("Failed to register LauncherStatus handler");

    cmd_router
        .register_singleton_handler::<DiscoveryCommand>()
        .await
        .expect("Failed to register DiscoveryCommand handler");

    cmd_router
        .register_singleton_handler::<SupervisorCommand>()
        .await
        .expect("Failed to register SupervisorCommand handler");

    let gql_server = GraphQLServer::new(
        db_arc.clone(),
        cmd_router.clone(),
        "0.0.0.0:8080".to_string(),
    )
    .expect("Failed to start GraphQL server");

    let mut gql_server_handle = gql_server
        .serve()
        .await
        .expect("Failed to start GraphQL server");

    let mut exec_mgr_registry = ExecutionManagerRegistry::new();

    exec_mgr_registry.register_manager(Arc::new(LocalProcessExecutionManager::new()));

    let mut coordinator = Coordinator::new(
        CoordinatorConfig { max_launchers: 10 },
        cmd_router.clone(),
        db_arc.clone(),
        Arc::new(exec_mgr_registry),
    )
    .await;

    tokio::spawn(async move {
        coordinator.run_main_loop().await.unwrap();
    });

    let mut registry_discovery = RegistryDiscoveryService::new(
        vec!["http://localhost:50051".to_string()],
        cmd_router.clone(),
        db_arc.clone(),
    )
    .await;

    let _registry_discovery_handle = tokio::spawn(async move {
        registry_discovery
            .run_loop()
            .await
            .expect("Failed to start registry discovery");
    });

    // TODO(thegenem0):
    // Handle graceful shutdown better.
    // - Spawn separate task for waiting on the ack
    // - Keep main task alive and free to handle other stuff

    let mut is_shutting_down = false;
    let mut master_shutdown_rx: Option<oneshot::Receiver<()>> = None;

    'main_loop: loop {
        if is_shutting_down {
            if let Some(rx) = master_shutdown_rx.take() {
                println!(
                    "Main Loop: Graceful shutdown process active, waiting for supervisor acknowledgment and finalization..."
                );

                tokio::select! {
                    biased;

                    _ = rx => {
                         println!("Main Loop: Supervisor acknowledgment sequence completed. Exiting application.");
                         break 'main_loop;
                    }

                    _ = tokio::time::sleep(Duration::from_secs(15)) => {
                         eprintln!("Main Loop: Overall timeout waiting for supervisor acknowledgment/finalization. Forcing exit.");
                         break 'main_loop;
                    }
                     _ = tokio::signal::ctrl_c() => {
                        eprintln!("Main Loop: Second Ctrl+C received during shutdown. Forcing exit immediately.");
                        break 'main_loop;
                    }
                }
            } else {
                eprintln!(
                    "Main Loop: In shutdown state but no signal to wait for. Exiting application."
                );
                break 'main_loop;
            }
        } else {
            tokio::select! {
                biased;

                ctrl_c_res = tokio::signal::ctrl_c() => {
                    if ctrl_c_res.is_err() {
                        eprintln!("Main Loop: Error listening for Ctrl+C: {:?}", ctrl_c_res.err());
                        continue;
                    }

                    is_shutting_down = true;
                    let(master_rx, _handle) = spawn_termination_task(cmd_router.clone()).await;
                    master_shutdown_rx = Some(master_rx);

                },

                gql_server_res = &mut gql_server_handle, if !is_shutting_down && !gql_server_handle.is_finished() => {
                     eprintln!("Main Loop: GraphQL server task has completed (Result: {:?}). Initiating graceful shutdown...", gql_server_res.is_ok());
                     is_shutting_down = true;

                    let (master_rx, _handle) = spawn_termination_task(cmd_router.clone()).await;
                    master_shutdown_rx = Some(master_rx);

                }
            }
        }
    }

    println!("Main Loop: Exiting application.");
}

async fn spawn_termination_task(
    cmd_router: Arc<CommandRouter>,
) -> (oneshot::Receiver<()>, JoinHandle<()>) {
    let (master_tx, master_rx) = oneshot::channel::<()>();

    let task_handle = tokio::spawn(async move {
        let (coord_ack_tx, coord_ack_rx) = tokio::sync::oneshot::channel::<bool>();

        if cmd_router
            .dispatch(
                SupervisorCommand::Terminate {
                    ack_chan: coord_ack_tx,
                },
                None,
            )
            .await
            .is_err()
        {
            eprintln!(
                "Shutdown Task (Ctrl+C): Failed to dispatch Terminate command to supervisor."
            );
        } else {
            println!("Shutdown Task (Ctrl+C): Waiting for supervisor ACK (timeout 5s)...");
            match tokio::time::timeout(Duration::from_secs(5), coord_ack_rx).await {
                Ok(Ok(true)) => {
                    println!("Shutdown Task (Ctrl+C): Supervisor acknowledged graceful shutdown.")
                }
                Ok(Ok(false)) => eprintln!(
                    "Shutdown Task (Ctrl+C): Supervisor acknowledged, but indicated shutdown was NOT fully graceful."
                ),
                Ok(Err(_e)) => eprintln!(
                    "Shutdown Task (Ctrl+C): Supervisor ACK channel closed prematurely; shutdown likely failed or was abrupt before ack."
                ),
                Err(_e) => eprintln!(
                    "Shutdown Task (Ctrl+C): Timeout waiting for supervisor's shutdown ACK."
                ),
            }
        }

        if master_tx.send(()).is_err() {
            eprintln!(
                "Shutdown Task (Ctrl+C): Failed to send completion signal to main loop (main loop might have already exited)."
            );
        }
    });

    (master_rx, task_handle)
}
