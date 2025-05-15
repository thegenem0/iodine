use iodine_common::error::Error;
use iodine_protobuf::{
    json_value_to_prost_struct,
    v1::{
        GetPipelineDefinitionsRequest, GetPipelineDefinitionsResponse, GetRegistryMetadataRequest,
        GetRegistryMetadataResponse, PipelineDefinitionProto, TaskDefinitionProto,
        TaskDependencyProto, pipeline_registry_service_server::PipelineRegistryService,
    },
};
use tonic::{Request, Response, Status};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{MASTER_SYSTEM_NAMESPACE, parse_yaml, schema::RegistryConfig};

#[derive(Debug)]
#[allow(dead_code)]
pub struct GrpcPipelineRegistryService {
    registry_id: Uuid,
    config: RegistryConfig,
}

impl GrpcPipelineRegistryService {
    pub fn new(pipeline_cfg_path: String) -> Result<Self, Error> {
        let yaml_file = std::fs::read_to_string(pipeline_cfg_path.clone()).map_err(|e| {
            error!(
                "Failed to read pipelines conf at {}: {}",
                pipeline_cfg_path, e
            );

            Error::Internal(format!(
                "Failed to read pipelines conf at {}: {}",
                pipeline_cfg_path, e
            ))
        })?;

        let parsed_config = parse_yaml(&yaml_file)?;

        let registry_id = Uuid::new_v5(
            &MASTER_SYSTEM_NAMESPACE,
            parsed_config.registry_identifier.as_bytes(),
        );

        Ok(Self {
            registry_id,
            config: parsed_config,
        })
    }
}

#[tonic::async_trait]
impl PipelineRegistryService for GrpcPipelineRegistryService {
    #[instrument(skip(self))]
    async fn get_registry_metadata(
        &self,
        _request: Request<GetRegistryMetadataRequest>,
    ) -> Result<Response<GetRegistryMetadataResponse>, Status> {
        info!(id = self.registry_id.to_string(), "GetRegistryMetadata");

        let response = GetRegistryMetadataResponse {
            registry_id: self.registry_id.to_string(),
            registry_metadata: Default::default(),
        };

        Ok(Response::new(response))
    }

    #[instrument(skip(self))]
    async fn get_pipeline_definitions(
        &self,
        _request: Request<GetPipelineDefinitionsRequest>,
    ) -> Result<Response<GetPipelineDefinitionsResponse>, Status> {
        info!(id = self.registry_id.to_string(), "GetPipelineDefinitions");

        let mut def_protos: Vec<PipelineDefinitionProto> = Vec::new();

        for def in self.config.pipelines.iter() {
            let mut task_defs: Vec<TaskDefinitionProto> = Vec::new();
            let task_deps: Vec<TaskDependencyProto> = Vec::new();

            for step in def.steps.iter() {
                let task_def_id = Uuid::new_v5(&self.registry_id, step.name.as_bytes());

                let task_def = TaskDefinitionProto {
                    id: task_def_id.to_string(),
                    name: step.name.clone(),
                    description: step.description.clone(),
                    config_schema: json_value_to_prost_struct(serde_json::Value::Object(
                        serde_json::Map::new(),
                    )),
                    user_code_metadata: json_value_to_prost_struct(serde_json::Value::Object(
                        serde_json::Map::new(),
                    )),
                };
                task_defs.push(task_def);
            }

            let task_dep_id = Uuid::new_v5(&self.registry_id, def.name.as_bytes());

            let proto = PipelineDefinitionProto {
                id: task_dep_id.to_string(),
                name: def.name.clone(),
                description: def.description.clone(),
                default_backend: json_value_to_prost_struct(serde_json::Value::String(
                    "test".to_string(),
                )),
                default_tags: json_value_to_prost_struct(serde_json::Value::Array(vec![
                    serde_json::Value::String("test".to_string()),
                    serde_json::Value::String("test2".to_string()),
                ])),
                metadata: json_value_to_prost_struct(serde_json::Value::Object(
                    serde_json::Map::new(),
                )),
                task_definitions: task_defs,
                task_dependencies: task_deps,
            };

            def_protos.push(proto);
        }

        let response = GetPipelineDefinitionsResponse {
            registry_id: self.registry_id.to_string(),
            registry_metadata: Default::default(),
            pipeline_definitions: def_protos,
            success: true,
            error_message: Default::default(),
        };

        Ok(Response::new(response))
    }
}
