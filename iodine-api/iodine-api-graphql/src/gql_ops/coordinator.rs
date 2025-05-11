use async_graphql::{Context, Error as GqlError, Object, Result as GqlResult};
use iodine_common::{coordinator::CoordinatorCommand, error::Error};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::server::ContextData;

pub struct QueryRoot;
pub struct MutationRoot;

#[Object]
impl QueryRoot {
    async fn get_coordinator_status(&self, _ctx: &Context<'_>) -> Result<String, String> {
        todo!()
    }
}

#[Object]
impl MutationRoot {
    async fn submit_pipeline(
        &self,
        ctx: &Context<'_>,
        pipeline_definition_id: String,
    ) -> GqlResult<String> {
        let ctx = ctx.data::<ContextData>().map_err(|e| {
            eprintln!("Failed to get ContextData: {:?}", e);
            GqlError::new("Internal server error: Could not access context data.")
        })?;

        let pipeline_def_uuid = Uuid::parse_str(&pipeline_definition_id).map_err(|e| {
            eprintln!("Failed to parse pipeline definition id: {:?}", e);
            GqlError::new("Internal server error: Failed to parse pipeline definition id.")
        })?;

        match ctx.db.get_pipeline_definition(pipeline_def_uuid).await {
            Ok(Some(_pipeline_def)) => {} // Pipeline definition found, proceed to send command
            Ok(None) => {
                return Err(GqlError::new(format!(
                    "Pipeline definition with ID '{}' not found.",
                    pipeline_definition_id
                )));
            }
            Err(db_err) => {
                eprintln!(
                    "Database error checking pipeline definition {}: {}",
                    pipeline_definition_id, db_err
                );
                return Err(GqlError::new(format!(
                    "Failed to verify pipeline definition: {}. Please try again later.",
                    pipeline_definition_id
                )));
            }
        }

        let (response_tx, response_rx) = oneshot::channel::<Result<Uuid, Error>>();

        let command = CoordinatorCommand::SubmitPipeline {
            pipeline_id: pipeline_def_uuid,
            run_id_override: None,
            response_oneshot: response_tx,
        };

        if let Err(send_err) = ctx.coordinator_cmd_tx.send(command).await {
            eprintln!(
                "Failed to send SubmitPipeline command to Coordinator: {}",
                send_err
            );
            return Err(GqlError::new(
                "Failed to submit pipeline: Coordinator service may be unavailable.",
            ));
        }

        match response_rx.await {
            Ok(Ok(pipeline_run_id)) => {
                // Successfully submitted
                // Coordinator created a run and sent back its `pipeline_id`
                Ok(pipeline_run_id.to_string())
            }
            Ok(Err(app_err)) => {
                // Coordinator processed the request but encountered an application error
                eprintln!(
                    "Coordinator returned an error for pipeline submission: {}",
                    app_err
                );
                Err(GqlError::new(format!(
                    "Pipeline submission failed: {}",
                    app_err
                )))
            }
            Err(recv_err) => {
                // Failed to receive a response from the Coordinator (e.g., oneshot channel dropped)
                eprintln!(
                    "Failed to receive response from Coordinator for pipeline submission: {}",
                    recv_err
                );
                Err(GqlError::new(
                    "No response from pipeline coordinator. The submission may have failed or is in an unknown state.",
                ))
            }
        }
    }
}
