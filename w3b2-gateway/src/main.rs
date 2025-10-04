mod cli;
mod config;
mod error;
mod grpc;
mod storage;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::{GatewayConfig, load_config};
use std::{fs::File, str::FromStr};
use tokio::signal;
use tracing::Level;
use tracing_subscriber::{
    Registry,
    filter::LevelFilter,
    fmt::{self, writer::MakeWriterExt},
    prelude::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. Parse CLI arguments ---
    let cli = Cli::parse();

    match cli.command {
        Commands::Run(run_cmd) => {
            // --- 2. Load configuration or use defaults ---
            let config = if let Some(config_path) = run_cmd.config {
                // We can't log yet, so we print directly.
                println!("Loading configuration from '{}'", &config_path);
                load_config(&config_path)?
            } else {
                println!("No config file provided, using default settings.");
                GatewayConfig::default()
            };

            // --- 3. Initialize logging based on config ---
            let log_level = Level::from_str(&config.gateway.log.level).unwrap_or(Level::INFO);
            let level_filter = LevelFilter::from_level(log_level);

            let subscriber = Registry::default().with(level_filter);

            // Configure based on output destination first
            if config.gateway.log.output == config::LogOutput::File {
                let file_path = config.gateway.log.file_path.as_deref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Log output is 'file' but 'file_path' is not specified in config"
                    )
                })?;
                let log_file = File::create(file_path)?;
                let file_writer = log_file.with_max_level(log_level);

                match config.gateway.log.format {
                    config::LogFormat::Plain => subscriber
                        .with(fmt::layer().with_writer(file_writer).pretty())
                        .init(),
                    config::LogFormat::Json => subscriber
                        .with(fmt::layer().with_writer(file_writer).json())
                        .init(),
                }
            } else {
                // Default to stdout
                let stdout_writer = std::io::stdout.with_max_level(log_level);
                match config.gateway.log.format {
                    config::LogFormat::Plain => {
                        let fmt_layer = fmt::layer().with_writer(stdout_writer).pretty();
                        subscriber.with(fmt_layer).init();
                    }
                    config::LogFormat::Json => {
                        let fmt_layer = fmt::layer().with_writer(stdout_writer).json();
                        subscriber.with(fmt_layer).init();
                    }
                }
            };

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
