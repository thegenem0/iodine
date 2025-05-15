use std::collections::HashMap;

use chrono::Utc;
use iodine_common::{
    error::Error,
    pipeline::{PipelineBackend, PipelineDefinition, PipelineInfo},
    task::{TaskDefinition, TaskDependency},
};
use iodine_protobuf::{
    prost_struct_to_json_value,
    v1::{
        GetPipelineDefinitionsRequest, GetRegistryMetadataRequest,
        pipeline_registry_service_client::PipelineRegistryServiceClient,
    },
};
use uuid::Uuid;

pub struct GrpcClient {
    /// Map of regitry IDs to their corresponding gRPC clients.
    inner_clients: HashMap<Uuid, PipelineRegistryServiceClient<tonic::transport::Channel>>,
}

impl GrpcClient {
    pub async fn new(registry_addresses: Vec<String>) -> Result<Self, Error> {
        let mut inner_clients = HashMap::new();

        for addr in registry_addresses {
            let mut client = PipelineRegistryServiceClient::connect(addr).await.unwrap();

            let res = client
                .get_registry_metadata(GetRegistryMetadataRequest::default())
                .await
                .map_err(|e| Error::Internal(format!("Failed to get registry metadata: {}", e)))?;

            let registry_id = Uuid::parse_str(res.into_inner().registry_id.as_str())
                .map_err(|e| Error::Internal(format!("Failed to parse registry ID: {}", e)))?;

            inner_clients.insert(registry_id, client);
        }

        Ok(Self { inner_clients })
    }

    pub async fn get_metadata(&self) -> Result<Vec<(Uuid, serde_json::Value)>, Error> {
        let mut clients: Vec<(Uuid, serde_json::Value)> = Vec::new();
        for (registry_id, cl) in self.inner_clients.iter() {
            let mut client = cl.clone();
            let res = client
                .get_registry_metadata(GetRegistryMetadataRequest::default())
                .await
                .map_err(|e| Error::Internal(format!("Failed to get registry metadata: {}", e)))?
                .into_inner();

            let json_value = if res.registry_metadata.is_some() {
                prost_struct_to_json_value(res.registry_metadata.unwrap())
            } else {
                serde_json::Value::Null
            };

            clients.push((*registry_id, json_value));
        }

        Ok(clients)
    }

    pub async fn get_pipeline_definitions(
        &self,
        registry_id: Uuid,
    ) -> Result<Vec<PipelineDefinition>, Error> {
        let mut client = self
            .inner_clients
            .get(&registry_id)
            .cloned()
            .ok_or_else(|| Error::Internal(format!("No client for registry ID {}", registry_id)))?;

        let res = client
            .get_pipeline_definitions(GetPipelineDefinitionsRequest::default())
            .await
            .map_err(|e| Error::Internal(format!("Failed to get pipeline definitions: {}", e)))?
            .into_inner();

        let mut pipeline_defs = Vec::new();

        for def in res.pipeline_definitions {
            let pipeline_id = Uuid::parse_str(def.id.as_str())
                .map_err(|e| Error::Internal(format!("Failed to parse pipeline ID: {}", e)))?;

            let now = Utc::now();
            let mut task_defs = Vec::new();
            let mut task_deps = Vec::new();

            for task_def in def.task_definitions {
                let task_id = Uuid::parse_str(task_def.id.as_str())
                    .map_err(|e| Error::Internal(format!("Failed to parse task ID: {}", e)))?;

                let task_def = TaskDefinition {
                    id: task_id,
                    pipeline_id,
                    name: task_def.name,
                    description: Some(task_def.description),
                    config_schema: task_def.config_schema.map(prost_struct_to_json_value),
                    user_code_metadata: task_def.user_code_metadata.map(prost_struct_to_json_value),
                };

                task_defs.push(task_def);
            }

            for task_dep in def.task_dependencies {
                let source_dep_id = Uuid::parse_str(task_dep.source_task_definition_id.as_str())
                    .map_err(|e| Error::Internal(format!("Failed to parse task ID: {}", e)))?;

                let target_dep_id = Uuid::parse_str(task_dep.target_task_definition_id.as_str())
                    .map_err(|e| Error::Internal(format!("Failed to parse task ID: {}", e)))?;

                let task_dep = TaskDependency {
                    pipeline_id,
                    source_task_definition_id: source_dep_id,
                    source_output_name: task_dep.source_output_name,
                    target_task_definition_id: target_dep_id,
                    target_input_name: task_dep.target_input_name,
                };

                task_deps.push(task_dep);
            }

            let pipeline_def = PipelineDefinition {
                info: PipelineInfo {
                    id: pipeline_id,
                    name: def.name,
                    description: def.description,
                    default_backend: PipelineBackend::Local,
                    default_tags: def.default_tags.map(prost_struct_to_json_value),
                    metadata: def.metadata.map(prost_struct_to_json_value),
                    created_at: now,
                    updated_at: now,
                },
                task_definitions: task_defs,
                task_dependencies: task_deps,
            };

            pipeline_defs.push(pipeline_def);
        }

        Ok(pipeline_defs)
    }
}
