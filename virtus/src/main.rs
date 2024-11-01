#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let virtus = virtus::Builder::new()
        .set_dir("/tmp/virtus")
        .bind("0.0.0.0".parse()?)
        .build()?;

    virtus.start().await?;

    Ok(())
}
