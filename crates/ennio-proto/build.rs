fn main() -> Result<(), Box<dyn std::error::Error>> {
    if option_env!("PROTOC").is_none() {
        // Safety: build scripts run single-threaded before any compilation,
        // so setting env vars here cannot race with other threads.
        unsafe {
            std::env::set_var("PROTOC", protobuf_src::protoc());
        }
    }
    tonic_build::compile_protos("proto/ennio_node.proto")?;
    Ok(())
}
