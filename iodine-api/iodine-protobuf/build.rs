fn main() -> Result<(), Box<dyn std::error::Error>> {
    let include_paths: &[_] = &["proto/"];

    if !std::path::Path::new(include_paths[0]).exists() {
        panic!("Proto directory does not exist at {}", include_paths[0]);
    }

    let proto_files: &[_] = &[
        "proto/iodine/v1/common.proto",
        "proto/iodine/v1/pipeline_registry.proto",
    ];

    for proto_file in proto_files {
        if !std::path::Path::new(proto_file).exists() {
            panic!("Proto file does not exist at {}", proto_file);
        }
    }

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(proto_files, include_paths)?;

    Ok(())
}
