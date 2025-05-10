use std::collections::HashMap;

use async_trait::async_trait;
use iodine_common::error::Error;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub pipeline_id: Uuid,
    pub run_id: Uuid,
    /// Preferred type of resource (e.g., "docker", "local_process", "cloud_run_job").
    /// A manager might support multiple or specialize in one.
    pub resource_type_preference: String,
    pub cpu_request: Option<String>,
    pub memory_request: Option<String>,
    pub image_uri: Option<String>,
    /// The primary command to execute within the provisioned environment.
    pub command_to_execute: Vec<String>,
    /// Arguments for the command.
    pub command_args: Vec<String>,
    /// Environment variables to set within the provisioned environment.
    pub environment_variables: HashMap<String, String>,
    /// Timeout for the execution of the primary command, in seconds.
    pub execution_timeout_seconds: Option<u64>,
    /// Labels or tags to apply to the provisioned resource for tracking.
    pub labels: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionedResourceDetails {
    /// A unique identifier for this specific instance of the provisioned resource
    /// (e.g., Docker container ID, local process PID, Cloud Run Job name).
    pub instance_id: String,
    /// The ID of the ResourceManager implementation that created this.
    pub manager_id: String,
    /// The type of resource that was provisioned (e.g., "docker", "local_process").
    pub resource_type_provisioned: String,
    /// Information specific to the type of resource, enabling interaction.
    pub connection_info: ResourceConnectionInfo,
    /// Any additional, manager-specific metadata that might be needed for operations like teardown
    /// but isn't directly used by the RunLauncher for execution.
    #[serde(default)]
    pub manager_specific_opaque_data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ResourceConnectionInfo {
    LocalProcess { pid: u32 },
    // DockerContainer {
    //     container_id: String,
    //     // Network details if the RunLauncher needs to connect to the container,
    //     // or if interaction is via 'docker exec'.
    //     // e.g. exposed ports: {"8080/tcp": "localhost:32768"}
    //     network_ports_map: Option<HashMap<String, String>>,
    // },
    // CloudRun {
    //     // For a Service, this would be the invocation URL.
    //     // For a Job, this might be the Job name for status polling or log retrieval.
    //     target_identifier: String,
    //     // Region where the resource is located.
    //     region: String,
    //     // Project ID.
    //     project_id: String,
    // },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProvisionedResourceStatus {
    Pending,                 // Resource is being provisioned.
    Initializing, // Resource is provisioned, but the environment/application inside is starting.
    Running,      // The primary command/application is actively running.
    Succeeded,    // The primary command/application completed successfully.
    Failed,       // The primary command/application failed.
    Terminating,  // Resource is in the process of being torn down.
    Terminated,   // Resource has been successfully torn down.
    Unknown(Option<String>), // Status cannot be determined or an error occurred.
    ErrorState(String), // Resource is in an unrecoverable error state.
}

#[async_trait]
pub trait ResourceManager: Send + Sync {
    /// Returns a unique identifier for this specific ResourceManager instance
    /// (e.g., "docker_default_pool", "gcp_us-central1_cloudrun_manager").
    fn manager_id(&self) -> &str;

    /// Indicates the primary type of resource this manager provisions
    /// (e.g., "docker", "local_process", "cloud_run_job").
    /// Used by the Coordinator select an appropriate manager.
    fn supported_resource_type(&self) -> String;

    /// Provisions a new execution environment based on the `ResourceRequest`.
    ///
    /// This method is responsible for allocating/creating the actual resource
    /// (e.g., starting a Docker container, spawning a local process, creating a Cloud Run Job).
    ///
    /// # Arguments
    /// * `request` - A reference to the `ResourceRequest` detailing the requirements.
    ///
    /// # Returns
    /// * `Ok(ProvisionedResourceDetails)` if successful, containing all necessary information
    ///   for the `RunLauncher` to use the resource and for the `Coordinator` to manage it.
    /// * `Err(Error)` if provisioning fails for any reason.
    async fn provision(
        &self,
        request: &ResourceRequest,
    ) -> Result<ProvisionedResourceDetails, Error>;

    /// Tears down/releases a previously provisioned resource.
    ///
    /// This method should ensure that all underlying infrastructure associated with
    /// the `instance_id` is properly cleaned up and released.
    ///
    /// # Arguments
    /// * `details` - The `ProvisionedResourceDetails` that were returned by `provision`.
    ///               This provides context like `instance_id` and any opaque data needed by the manager.
    ///
    /// # Returns
    /// * `Ok(())` if teardown was successful or the resource was already gone.
    /// * `Err(Error)` if teardown fails.
    async fn teardown(&self, details: &ProvisionedResourceDetails) -> Result<(), Error>;

    /// Fetches the current status of a provisioned resource.
    ///
    /// This can be used by the `Coordinator` to monitor the health/progress of the
    /// underlying infrastructure if the `RunLauncher` itself isn't providing sufficient status,
    /// or for out-of-band checks.
    ///
    /// # Arguments
    /// * `details` - The `ProvisionedResourceDetails` of the resource to check.
    ///
    /// # Returns
    /// * `Ok(ProvisionedResourceStatus)` with the current status.
    /// * `Err(Error)` if the status cannot be fetched.
    async fn get_status(
        &self,
        details: &ProvisionedResourceDetails,
    ) -> Result<ProvisionedResourceStatus, Error>;
}
