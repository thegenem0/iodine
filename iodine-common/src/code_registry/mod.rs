use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeRegistry {
    pub id: Uuid,
    pub name: String,
    pub source_type: CodeRegistrySource,

    /// Git URL/branch/commit, path, registry info
    pub metadata: serde_json::Value,
    pub status: CodeRegistryStatus,
    pub status_message: Option<String>,
    pub last_updated: DateTime<Utc>,
    pub last_loaded: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CodeRegistrySource {
    Git,
    LocalPath,
    Registry,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeRegistryStatus {
    /// Syncing or parsing
    Loading,
    /// Definitions successfully loaded
    Loaded,
    /// Definitions failed to load
    Error,
}
