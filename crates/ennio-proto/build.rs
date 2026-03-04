fn main() -> Result<(), Box<dyn std::error::Error>> {
    if option_env!("PROTOC").is_none() {
        #[cfg(feature = "protobuf-src")]
        {
            // Safety: build scripts run single-threaded before any compilation,
            // so setting env vars here cannot race with other threads.
            unsafe {
                std::env::set_var("PROTOC", protobuf_src::protoc());
            }
        }
        #[cfg(not(feature = "protobuf-src"))]
        {
            return Err(
                "PROTOC environment variable not set and protobuf-src feature not enabled. \
                Install protobuf-compiler or enable the protobuf-src feature."
                    .into(),
            );
        }
    }
    tonic_build::compile_protos("proto/ennio_node.proto")?;
    Ok(())
}
