// w3b2-connector/src/config.rs

use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::CommitmentLevel;

/// Represents the main configuration for the w3b2-connector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub solana: Solana,
    #[serde(default)]
    pub synchronizer: Synchronizer,
    #[serde(default)]
    pub storage: Storage,
    #[serde(default)]
    pub logging: Logging,
    #[serde(default)]
    pub grpc_server: Server,
    #[serde(default)]
    pub p2p_server: Server,
}

/// Solana network connection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Solana {
    pub rpc_url: String,
    pub ws_url: String,
    // Field is no longer an Option
    #[serde(with = "serde_commitment")]
    pub commitment: CommitmentLevel,
}

/// Settings for the event synchronizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Synchronizer {
    pub max_catchup_depth: Option<u64>,
    pub poll_interval_secs: u64,
    pub max_signature_fetch: usize,
}

/// Storage and logging directory settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Storage {
    pub data_dir: String,
    pub log_dir: String,
}

/// Logging settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Logging {
    pub log_format: String,
    pub log_level: String,
}

/// Generic server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Server {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

// --- Default Implementations ---

impl Default for Config {
    fn default() -> Self {
        Self {
            solana: Solana::default(),
            synchronizer: Synchronizer::default(),
            storage: Storage::default(),
            logging: Logging::default(),
            grpc_server: Server {
                enabled: true,
                host: "[::1]".to_string(),
                port: 50051,
            },
            p2p_server: Server {
                enabled: false,
                host: "0.0.0.0".to_string(),
                port: 60061,
            },
        }
    }
}

impl Default for Solana {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8899".to_string(),
            ws_url: "ws://127.0.0.1:8900".to_string(),
            commitment: CommitmentLevel::Confirmed,
        }
    }
}

impl Default for Synchronizer {
    fn default() -> Self {
        Self {
            max_catchup_depth: None,
            poll_interval_secs: 3,
            max_signature_fetch: 1000,
        }
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            data_dir: "./w3b2_data".to_string(),
            log_dir: "logs".to_string(),
        }
    }
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            log_format: "plain".to_string(),
            log_level: "INFO".to_string(),
        }
    }
}

// ADDED: Default implementation for the Server struct.
// This is required by `#[serde(default)]` on the `Config` struct's fields.
impl Default for Server {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 0,
        }
    }
}

// FIXED: This module now works directly with CommitmentLevel, not Option<...>.
mod serde_commitment {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(c: &CommitmentLevel, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match c {
            CommitmentLevel::Processed => "Processed",
            CommitmentLevel::Confirmed => "Confirmed",
            CommitmentLevel::Finalized => "Finalized",
        };
        serializer.serialize_str(s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<CommitmentLevel, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let level = match s.to_lowercase().as_str() {
            "processed" => CommitmentLevel::Processed,
            "confirmed" => CommitmentLevel::Confirmed,
            "finalized" => CommitmentLevel::Finalized,
            // Fallback to Confirmed if the string is unrecognized
            _ => CommitmentLevel::Confirmed,
        };
        Ok(level)
    }
}
