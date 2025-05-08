use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunOrdering {
    pub field: RunOrderingField,
    pub direction: SortDirection,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunOrderingField {
    ExecutionId,
    DefinitionId,
    Status,
    StartTime,
    EndTime,
    CreatedAt,
    UpdatedAt,
}
