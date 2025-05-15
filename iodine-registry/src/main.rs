pub mod parser;
#[allow(dead_code)]
pub mod schema;
pub mod server;

use std::net::SocketAddr;

use clap::{Arg, Command};
use iodine_common::error::Error;
use iodine_protobuf::v1::pipeline_registry_service_server::PipelineRegistryServiceServer;
pub use parser::parse_yaml;
use server::GrpcPipelineRegistryService;
use tonic::transport::Server;
use tracing::{error, info};
use uuid::Uuid;

const MASTER_SYSTEM_NAMESPACE: Uuid = uuid::uuid!("1d33bb48-4746-4e0b-b93f-f9eebe90e1cb");

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_level(true)
        .with_target(true)
        .init();

    let matches = Command::new("iodine")
        .about("Iodine Registry CLI")
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("serve")
                .about("Starts the Iodine Registry Server")
                .arg(
                    Arg::new("listen_addr")
                        .short('l')
                        .long("listen_addr")
                        .help("Address to listen on")
                        .default_value("[::1]:50051")
                        .action(clap::ArgAction::Set),
                )
                .arg(
                    Arg::new("config")
                        .short('c')
                        .long("config")
                        .help("Path to the configuration file")
                        .default_value("./config.yaml")
                        .action(clap::ArgAction::Set),
                )
                .arg(
                    Arg::new("pipeline_config")
                        .short('p')
                        .long("pipeline_config")
                        .help("Path to the pipeline configuration file")
                        .default_value("./pipelines.yaml")
                        .action(clap::ArgAction::Set),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("serve", sub_matches)) => {
            let listen_addr = sub_matches.get_one::<String>("listen_addr").unwrap();
            let _config_path = sub_matches.get_one::<String>("config").unwrap();
            let pipeline_config_path = sub_matches.get_one::<String>("pipeline_config").unwrap();

            let service = GrpcPipelineRegistryService::new(pipeline_config_path.to_string())
                .expect("Failed to create service");

            let parsed_listen_addr: SocketAddr = listen_addr
                .parse()
                .map_err(|e| Error::Internal(format!("Failed to parse listen address: {}", e)))?;

            info!("Starting Iodine Registry Server on {}", parsed_listen_addr);

            let server_handle = tokio::spawn(async move {
                Server::builder()
                    .add_service(PipelineRegistryServiceServer::new(service))
                    .serve(parsed_listen_addr)
                    .await
            });

            tokio::select! {
                _ = server_handle => {
                    error!("Server Handle Error");
                }

                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl-C, shutting down server");
                }
            }
        }
        _ => {
            println!("Invalid subcommand");
        }
    }

    Ok(())
}
