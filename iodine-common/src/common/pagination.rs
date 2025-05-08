use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub limit: u64,
    pub offset: u64,
}
