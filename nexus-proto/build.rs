fn main() -> Result<(), Box<dyn std::error::Error>> {
    let has_system_protoc = std::env::var("PROTOC").is_ok()
        || std::process::Command::new("protoc")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    if !has_system_protoc {
        let protoc = protoc_bin_vendored::protoc_bin_path()?;
        std::env::set_var("PROTOC", protoc);
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/game_server.proto"], &["proto"])?;
    Ok(())
}
