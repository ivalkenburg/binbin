//! Ephemeral test fixture; not included in the production binary.
use binbin_core::App;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let app = App::open(dir.path(), "http://localhost:8086".into())?;
    app.create_admin("admin", "browser-test-password")?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8086").await?;
    axum::serve(listener, binbin_server::router(app)).await?;
    Ok(())
}
