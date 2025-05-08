use async_trait::async_trait;
use std::fmt::Debug;

use crate::error::Error;

#[async_trait]
pub trait BaseDbTrait: Send + Sync + Debug + 'static {
    /// Logs a generic system event not tied to specific state updates.
    /// ---
    /// Should be used sparingly.
    /// Logging should happen as part of every database call,
    /// as part of a transaction, and failures should be handled
    /// appropriately within the given database call.
    async fn log_system_event(&self, message: String) -> Result<(), Error>;
}
