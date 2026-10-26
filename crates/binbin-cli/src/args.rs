use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Binbin file-sharing client")]
pub(super) struct Args {
    #[arg(long, env = "BINBIN_URL", global = true)]
    pub(super) server: Option<String>,
    #[arg(
        long,
        global = true,
        help = "Read a token from this file; never persist another copy"
    )]
    pub(super) token_file: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "Read a token from stdin; never persist it"
    )]
    pub(super) token_stdin: bool,
    #[arg(
        long,
        global = true,
        help = "Read a token using a hidden prompt without persistence"
    )]
    pub(super) prompt_token: bool,
    #[arg(long, global = true)]
    pub(super) json: bool,
    #[command(subcommand)]
    pub(super) command: Command,
}
#[derive(Subcommand)]
pub(super) enum Command {
    /// Save a dashboard-generated API token in the OS credential store.
    Login,
    /// Remove the locally stored API token.
    Logout,
    /// Upload a batch, then publish it. Repeat with --resume after interruption.
    Upload(UploadArgs),
    List,
    Inspect {
        id: String,
    },
    Revoke {
        id: String,
    },
    Delete {
        id: String,
    },
    /// Download through authenticated management access (does not use allowance).
    Download {
        id: String,
        #[arg(long, default_value = "zip")]
        resource: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Download a public share (uses its allowance). Retries reuse local state.
    Fetch {
        url: String,
        #[arg(long)]
        resource: Option<String>,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        password: bool,
    },
}

#[derive(clap::Args)]
pub(crate) struct UploadArgs {
    #[arg(required = true)]
    pub(crate) files: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) title: Option<String>,
    #[arg(long, conflicts_with = "no_expiry")]
    pub(crate) days: Option<u32>,
    #[arg(long)]
    pub(crate) no_expiry: bool,
    #[arg(long)]
    pub(crate) downloads: Option<u64>,
    #[arg(long, help = "Prompt for a share password")]
    pub(crate) password: bool,
    #[arg(long, help = "Resume this draft ID")]
    pub(crate) resume: Option<String>,
}
