use iodine_protobuf::v1::{ExecutionContextProto, execution_context_proto::ContextVariant};
use std::time::Duration;

use iodine_common::resource_manager::{ExecutionContext, LocalProcessExecutionContext};

pub(crate) fn map_exec_ctx_to_domain(exec_ctx: ExecutionContextProto) -> ExecutionContext {
    match exec_ctx.context_variant {
        Some(ContextVariant::LocalProcess(local_process)) => {
            ExecutionContext::LocalProcess(LocalProcessExecutionContext {
                entry_point: local_process.entry_point,
                args: local_process.args,
                env_vars: local_process.env_vars,
                exec_timeout: local_process
                    .exec_timeout
                    .map(|t| Duration::from_secs(t.seconds as u64)),
            })
        }
        _ => unimplemented!(),
    }
}
