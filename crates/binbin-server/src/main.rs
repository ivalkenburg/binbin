#[tokio::main]
async fn main() -> anyhow::Result<()> {
    binbin_server::run().await
}
