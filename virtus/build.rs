use anyhow::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["proto/virtus.proto"], &["proto"])?;

    Ok(())
}
