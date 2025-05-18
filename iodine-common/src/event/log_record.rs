use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use uuid::Uuid;

// It is used by strum to convert the enum to a string
// but the compiler complains that it is unused
#[allow(unused_imports)]
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventLogRecord {
    pub event_id: i64,
    pub run_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum EventType {
    // Pipeline lifecycle
    DefinitionRegistered,
    DefinitionUpdated,
    DefinitionDeregistered,

    // Run lifecycle
    RunEnqueued,
    RunDequeued,
    RunStarting,
    RunStart,
    RunSuccess,
    RunFailure,
    RunCanceling,
    RunCanceled,

    // Task lifecycle
    TaskPending,
    TaskQueued,
    TaskStart,
    TaskSuccess,
    TaskFailure,
    TaskSkipped,
    TaskCancelled,
    TaskReadyToRetry,
    TaskInput,
    TaskOutput,
    TaskExpectationResult,

    // Engine/system events
    EngineEvent,
    EngineInitFailure,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let s = "RunStart";
        let e = EventType::from_str(s).unwrap();
        assert_eq!(e, EventType::RunStart);
    }

    #[test]
    fn test_to_string() {
        let s = "RunStart";
        let e = EventType::from_str(s).unwrap();
        assert_eq!(e.to_string(), s);
    }
}
