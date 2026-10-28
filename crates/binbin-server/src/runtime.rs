use binbin_core::App;
use clap::{Parser, Subcommand};
#[derive(Parser)]
#[command(version, about = "Private file sharing")]
struct Args {
    #[arg(long, env = "BINBIN_DATA", default_value = "data")]
    data: std::path::PathBuf,
    #[arg(
        long,
        env = "BINBIN_PUBLIC_URL",
        default_value = "http://localhost:8080"
    )]
    public_url: String,
    #[command(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    /// Start the HTTP server.
    Serve {
        #[arg(long, env = "BINBIN_BIND", default_value = "0.0.0.0:8080")]
        bind: String,
    },
    /// Create the first administrator. The password is read interactively.
    InitAdmin {
        #[arg(long)]
        username: String,
    },
    /// Reclaim expired files and abandoned drafts once.
    Cleanup,
    /// Consistent SQLite backup. Stop uploads while backing up files as documented.
    BackupDb { destination: std::path::PathBuf },
}
pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "binbin=info".into()),
        )
        .init();
    let args = Args::parse();
    let app = App::open(args.data, args.public_url)?;
    match args.command.unwrap_or(Command::Serve {
        bind: std::env::var("BINBIN_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into()),
    }) {
        Command::InitAdmin { username } => {
            let p =
                rpassword::prompt_password("Administrator password (at least 12 characters): ")?;
            let confirm = rpassword::prompt_password("Repeat password: ")?;
            anyhow::ensure!(p == confirm, "Passwords do not match");
            app.create_admin(&username, &p)?;
            println!("Administrator {username} created");
        }
        Command::Cleanup => app.cleanup()?,
        Command::BackupDb { destination } => {
            anyhow::ensure!(!destination.exists(), "Destination already exists");
            app.backup_database(&destination)?;
            println!("Database backed up");
        }
        Command::Serve { bind } => {
            app.recover_publications()?;
            app.cleanup()?;
            let cleanup = app.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    let app = cleanup.clone();
                    if let Err(e) = tokio::task::spawn_blocking(move || app.cleanup())
                        .await
                        .unwrap_or_else(|e| Err(binbin_core::Error(500, e.to_string())))
                    {
                        tracing::error!("cleanup: {e}");
                    }
                }
            });
            let listener = tokio::net::TcpListener::bind(&bind).await?;
            tracing::info!("Binbin listening on {bind}");
            axum::serve(listener, crate::router(app))
                .with_graceful_shutdown(async {
                    #[cfg(unix)]
                    {
                        let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("install SIGTERM handler");
                        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
                    }
                    #[cfg(not(unix))]
                    { let _ = tokio::signal::ctrl_c().await; }
                })
                .await?;
        }
    }
    Ok(())
}
