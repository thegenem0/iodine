use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use iodine_api_grpc::client::GrpcClient;
use iodine_common::{
    command::{CommandRouter, DiscoveryCommand},
    error::Error,
    state::DatabaseTrait,
};
use uuid::Uuid;

pub struct RegistryDiscoveryService {
    /// List of registry addresses to connect to.
    registry_addresses: HashSet<String>,

    /// Map of registry IDs to their corresponding gRPC clients.
    tracked_registries: HashMap<Uuid, GrpcClient>,

    /// Command router for dispatching commands.
    command_router: Arc<CommandRouter>,

    /// Database to store tracked registries.
    db: Arc<dyn DatabaseTrait>,
}

impl RegistryDiscoveryService {
    pub async fn new(
        registry_addresses: Vec<String>,
        command_router: Arc<CommandRouter>,
        db: Arc<dyn DatabaseTrait>,
    ) -> Self {
        Self {
            registry_addresses: registry_addresses.into_iter().collect(),
            tracked_registries: HashMap::new(),
            command_router,
            db,
        }
    }

    pub async fn run_loop(&mut self) -> Result<(), Error> {
        let mut command_rx = self
            .command_router
            .subscribe::<DiscoveryCommand>(None)
            .await?;

        loop {
            tokio::select! {
                command = command_rx.recv() => {
                    match command {
                        Some(DiscoveryCommand::InitRegistries) => {
                            self.setup_connections().await?;
                            self.load_pipelines(None).await?;
                        }
                        Some(DiscoveryCommand::ReloadRegistries) => {
                            self.load_pipelines(None).await?;
                        }

                        Some(DiscoveryCommand::ReloadRegistry { registry_id }) => {
                            self.load_pipelines(Some(registry_id)).await?;
                        }
                        Some(DiscoveryCommand::RemoveTrackedRegistry { registry_id }) => {
                            match self.tracked_registries.get(&registry_id) {
                                Some(client) => {
                                    let addr = client.get_metadata().address;
                                    self.tracked_registries.remove(&registry_id);
                                    self.registry_addresses.remove(&addr);
                                }
                                None => {
                                    eprintln!("Failed to remove tracked registry with ID {}: Not found.", registry_id);
                                }
                            }

                        }
                        // Command should never be `None`
                        None => unreachable!(),
                        _ => unimplemented!(),
                    }
                }
            }
        }
    }

    pub async fn setup_connections(&mut self) -> Result<(), Error> {
        let mut failed_ids = Vec::new();

        for addr in self.registry_addresses.iter() {
            match GrpcClient::new(addr.clone()).await {
                Ok(client) => {
                    let registry_id = client.get_metadata().id;
                    self.tracked_registries.insert(registry_id, client);
                }
                Err(e) => {
                    eprintln!("Failed to connect to registry at {}: {}", addr, e);
                    failed_ids.push(addr.clone());
                }
            }
        }

        Ok(())
    }

    async fn load_pipelines(&self, registry_id: Option<Uuid>) -> Result<(), Error> {
        if let Some(registry_id) = registry_id {
            let client = self
                .tracked_registries
                .get(&registry_id)
                .ok_or_else(|| Error::Internal("Registry not found.".to_string()))?;

            let pipelines = client.get_pipeline_definitions().await?;
            for pipeline in pipelines {
                self.db.register_pipeline(&pipeline).await?;
            }
        } else {
            for client in self.tracked_registries.values() {
                let pipelines = client.get_pipeline_definitions().await?;
                for pipeline in pipelines {
                    self.db.register_pipeline(&pipeline).await?;
                }
            }
        }

        Ok(())
    }
}
