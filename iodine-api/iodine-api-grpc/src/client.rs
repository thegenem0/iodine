use chrono::Utc;
use iodine_common::{
    code_registry::{CodeRegistrySource, PipelineRegistry},
    error::Error,
    pipeline::{PipelineBackend, PipelineDefinition, PipelineInfo},
    task::TaskDefinition,
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
    registry: PipelineRegistry,
    inner_client: PipelineRegistryServiceClient<tonic::transport::Channel>,
}

impl GrpcClient {
    pub async fn new(registry_address: String) -> Result<Self, Error> {
        let mut client = PipelineRegistryServiceClient::connect(registry_address.clone())
            .await
            .map_err(|e| Error::Internal(format!("Failed to connect to registry: {}", e)))?;

        let res = client
            .get_registry_metadata(GetRegistryMetadataRequest::default())
            .await
            .map_err(|e| Error::Internal(format!("Failed to get registry metadata: {}", e)))?
            .into_inner();

        let registry_id = Uuid::parse_str(res.registry_id.as_str())
            .map_err(|e| Error::Internal(format!("Failed to parse registry ID: {}", e)))?;

        let json_meta = if res.registry_metadata.is_some() {
            prost_struct_to_json_value(res.registry_metadata.unwrap())
        } else {
            serde_json::Value::Null
        };

        let registry = PipelineRegistry {
            id: registry_id,
            name: "Registry".to_string(),
            source_type: CodeRegistrySource::Registry,
            metadata: json_meta,
            address: registry_address,
        };

        Ok(Self {
            registry,
            inner_client: client,
        })
    }

    pub fn get_metadata(&self) -> PipelineRegistry {
        self.registry.clone()
    }

    pub async fn get_pipeline_definitions(&self) -> Result<Vec<PipelineDefinition>, Error> {
        let res = self
            .inner_client
            .clone()
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

            for task_def in def.task_definitions {
                let task_id = Uuid::parse_str(task_def.id.as_str())
                    .map_err(|e| Error::Internal(format!("Failed to parse task ID: {}", e)))?;

                let depends_on_ids = task_def
                    .depends_on
                    .iter()
                    .map(|id| Uuid::parse_str(id.as_str()))
                    .collect::<Result<Vec<Uuid>, _>>()
                    .map_err(|e| Error::Internal(format!("Failed to parse task ID: {}", e)))?;

                let task_def = TaskDefinition {
                    id: task_id,
                    pipeline_id,
                    name: task_def.name,
                    description: Some(task_def.description),
                    config_schema: task_def.config_schema.map(prost_struct_to_json_value),
                    user_code_metadata: task_def.user_code_metadata.map(prost_struct_to_json_value),
                    depends_on: depends_on_ids,
                };

                task_defs.push(task_def);
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
            };

            pipeline_defs.push(pipeline_def);
        }

        Ok(pipeline_defs)
    }
}
