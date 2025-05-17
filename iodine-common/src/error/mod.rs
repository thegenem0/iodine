use std::fmt::Debug;

mod worker;
pub use worker::WorkerError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Database Error: {0}")]
    Database(String),

    #[error("Serialization Error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Resource Not Found: {resource_type} with ID {resource_id}")]
    NotFound {
        resource_type: String,
        resource_id: String,
    },

    #[error("Invalid Input: {0}")]
    InvalidInput(String),

    #[error("State Transition Error: {0}")]
    StateTransition(String),

    #[error("gRPC Communication Error: {0}")]
    GrpcComm(String),

    #[error("Task Execution Error: {0}")]
    TaskExecution(String),

    #[error("Channel Communication Error: {0}")]
    ChannelComm(String),

    #[error("Configuration Error: {0}")]
    Config(String),

    #[error("Internal Error: {0}")]
    Internal(String),

    #[error("Terminate Signal Error")]
    TerminateSignal,

    #[error("Conflict Error: {0}")]
    Conflict(String),
}
