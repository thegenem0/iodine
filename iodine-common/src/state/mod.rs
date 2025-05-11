mod base;
mod coordinator;
mod launcher;
mod pipeline;
mod task;

pub use base::BaseDbTrait;
pub use coordinator::CoordinatorDbTrait;
pub use launcher::LauncherDbTrait;
pub use pipeline::PipelineDbTrait;
pub use task::TaskDbTrait;

/// Combined trait for all database operations
/// (Core, External, Common)
/// Should be used through dyn dispatch at the top level
/// to pass the complete database interface
#[allow(dead_code)]
pub trait DatabaseTrait:
    CoordinatorDbTrait + LauncherDbTrait + PipelineDbTrait + TaskDbTrait
{
}
