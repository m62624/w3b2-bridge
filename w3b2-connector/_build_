fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(true).compile(
        &["../w3b2-bridge-program/proto/events.proto"], // The file to compile
        &["../w3b2-bridge-program/proto"],              // The directory to search in
    )?;
    Ok(())
}
