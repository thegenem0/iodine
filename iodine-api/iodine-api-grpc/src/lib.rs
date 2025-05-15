pub mod client;

// use tonic::{Request, Response, Status};
// use uuid::Uuid;
//
// #[derive(Debug, Clone)]
// pub struct CodeServerConfig {
//     pub registry_id: Uuid,
//     pub language: String,
//     pub args: Vec<String>,
//     pub workdir: String,
//     pub registry_version: Option<String>,
// }
//
// #[derive(Debug)]
// pub struct GrpcCodeServerService {
//     config: CodeServerConfig,
// }
//
// impl GrpcCodeServerService {
//     pub fn new(config: CodeServerConfig) -> Self {
//         Self { config }
//     }
// }
//
// #[tonic::async_trait]
// impl PipelineRegistryService for GrpcCodeServerService {
//     async fn get_pipeline_definitions(
//         &self,
//         _request: Request<GetPipelineDefinitionsRequest>,
//     ) -> Result<Response<GetPipelineDefinitionsResponse>, Status> {
//         todo!()
//     }
// }
