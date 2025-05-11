use async_trait::async_trait;

use super::BaseDbTrait;

#[async_trait]
pub trait LauncherDbTrait: BaseDbTrait {}
