use clap::{Parser, Subcommand};

/// The main CLI structure for the W3B2 Gateway.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Defines the available subcommands for the application.
///
/// For now, we only have the `run` command to start the service.
/// Later, we can add commands like `cards`, `health`, etc.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the W3B2 Gateway service.
    /// This starts the Solana event listener and the gRPC server.
    Run(RunCmd),
}

/// Arguments for the `run` subcommand.
#[derive(Parser, Debug)]
pub struct RunCmd {
    /// Path to the gateway configuration TOML file.
    /// If not provided, default values will be used.
    #[arg(short, long)]
    pub config: Option<String>,
}
