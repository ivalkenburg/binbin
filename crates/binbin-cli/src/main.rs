#[tokio::main]
async fn main() -> anyhow::Result<()> {
    binbin_cli::run().await
}
