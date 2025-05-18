use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct RegistryConfig {
    pub version: String,
    pub registry_identifier: String,
    pub pipelines: Vec<Pipeline>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Pipeline {
    pub id: String,
    pub description: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Step {
    pub id: String,
    pub description: String,
    pub environment: Vec<KvPair>,
    pub labels: Vec<KvPair>,
    pub executor_config: ExecutorConfig,
    pub max_attempts: Option<i32>,
    pub working_directory: String,
    pub entrypoint: String,
    pub depends_on: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorConfig {
    LocalProcess(LocalProcessConfig),
    Docker(DockerConfig),
    CloudRunJob(CloudRunJobConfig),
}

#[derive(Debug, Deserialize, Clone)]
pub struct LocalProcessConfig {
    pub entry_point: Vec<String>,
    pub args: Vec<String>,
    pub env_vars: Vec<KvPair>,
    pub cpu_request: Option<String>,
    pub memory_request: Option<String>,
    pub timeout_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DockerConfig {
    pub image_uri: String,
    pub entrypoint: Vec<String>,
    pub timeout_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CloudRunJobConfig {
    pub region: String,
    pub project_id: String,
    pub job_name: String,
    pub timeout_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct KvPair {
    pub key: String,
    pub value: String,
}
