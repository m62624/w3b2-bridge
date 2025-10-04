mod cli;
mod config;
mod error;
mod grpc;
mod storage;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::load_config;
use tokio::signal;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Initialize logging ---
    tracing_subscriber::registry().init();

    // --- 2. Parse CLI arguments ---
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(run_cmd) => {
            // --- 3. Load configuration ---
            let config = load_config(&run_cmd.config)?;

            // --- 4. Start the main application logic ---
            let event_manager_handle = grpc::start(&config).await?;

            // --- 5. Wait for a shutdown signal ---
            match signal::ctrl_c().await {
                Ok(()) => {
                    tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
                    event_manager_handle.stop().await;
                    tracing::info!("Shutdown complete.");
                }
                Err(err) => {
                    tracing::error!(error = %err, "Failed to listen for shutdown signal.");
                }
            }
        }
    }

    Ok(())
}
