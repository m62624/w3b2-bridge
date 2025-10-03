use anyhow::{Context, Result};
use serde::Deserialize;
// Import the core configuration from our library
use w3b2_connector::config::ConnectorConfig;

/// The top-level configuration for the W3B2 Gateway application.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GatewayConfig {
    /// Configuration for the underlying connector library.
    #[serde(default)]
    pub connector: ConnectorConfig,
    /// Configuration specific to the gateway service.
    #[serde(default)]
    pub gateway: GatewaySpecificConfig,
}

/// Contains settings that are unique to the gateway binary.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GatewaySpecificConfig {
    /// The file path for the Sled database.
    pub db_path: String,
    /// gRPC server settings.
    #[serde(default)]
    pub grpc: GrpcConfig,
}

/// gRPC server connection settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GrpcConfig {
    pub host: String,
    pub port: u16,
}

// --- Default Implementations ---

impl Default for GatewaySpecificConfig {
    fn default() -> Self {
        Self {
            db_path: "./w3b2_gateway.db".to_string(),
            grpc: GrpcConfig::default(),
        }
    }
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 50051,
        }
    }
}

/// Loads the gateway configuration from a specified TOML file.
///
/// It uses the `config` crate to read the file and deserialize it into
/// the `GatewayConfig` struct.
pub fn load_config(path: &str) -> Result<GatewayConfig> {
    let builder = config::Config::builder()
        .add_source(config::File::with_name(path))
        .add_source(config::Environment::with_prefix("W3B2").separator("__"));

    let settings: GatewayConfig = builder
        .build()
        .context(format!("Failed to build configuration from '{}'", path))?
        .try_deserialize()
        .context("Failed to deserialize configuration")?;

    Ok(settings)
}
