use std::{collections::HashMap, env, sync::Arc, time::Duration};

use coordinator::{CoordinatorConfig, default::Coordinator};
use iodine_api_graphql::server::GraphQLServer;
use iodine_common::state::DatabaseTrait;
use iodine_persistence_pg::db::PostgresStateDb;

#[allow(dead_code)]
mod coordinator;
#[allow(dead_code)]
mod launcher;
#[allow(dead_code)]
mod resource_manager;

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let state_db = PostgresStateDb::new(&database_url)
        .await
        .expect("Failed to connect to database");

    let db_arc: Arc<dyn DatabaseTrait> = Arc::new(state_db);

    let (coordinator_cmd_tx, coordinator_cmd_rx) = tokio::sync::mpsc::channel(10);

    // Supervisor command channels
    let (supervisor_cmd_tx, supervisor_cmd_rx) = tokio::sync::mpsc::channel(1);

    let gql_server = GraphQLServer::new(
        db_arc.clone(),
        coordinator_cmd_tx,
        "0.0.0.0:8080".to_string(),
    )
    .expect("Failed to start GraphQL server");

    let mut gql_server_handle = gql_server
        .serve()
        .await
        .expect("Failed to start GraphQL server");

    let mut coordinator = Coordinator::new(
        CoordinatorConfig { max_launchers: 10 },
        db_arc.clone(),
        Arc::new(HashMap::new()),
        coordinator_cmd_rx,
        supervisor_cmd_rx,
    );

    tokio::spawn(async move {
        coordinator.run_main_loop().await.unwrap();
    });

    // TODO(thegenem0):
    // Handle graceful shutdown better.
    // - Spawn separate task for waiting on the ack
    // - Keep main task alive and free to handle other stuff

    tokio::select! {
        biased;
        _ = tokio::signal::ctrl_c() => {
            let (coord_ack_tx, coord_ack_rx) = tokio::sync::oneshot::channel::<bool>();

            if supervisor_cmd_tx.send(coordinator::SupervisorCommand::InitiateShutdown{ ack_channel: coord_ack_tx }).await.is_err() {
                eprintln!("Failed to send shutdown command to Coordinator. It might have already stopped.");
            }

            if coord_ack_rx.await.is_err() {
                eprintln!("Coordinator failed to shutdown gracefully. Retrying...");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        },
        _ = &mut gql_server_handle => {
            eprintln!("GraphQL server task has completed.");
            let (ack_tx, _) = tokio::sync::oneshot::channel::<bool>(); // Can ignore this ack for unexpected stop
            supervisor_cmd_tx.send(coordinator::SupervisorCommand::InitiateShutdown { ack_channel: ack_tx }).await.ok();
        }
    }
}
