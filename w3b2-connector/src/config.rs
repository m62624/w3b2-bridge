#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use solana_sdk::commitment_config::CommitmentLevel;

/// Represents the core configuration required by the w3b2-connector library.
/// This struct should be created by the user of the library and passed to the EventManager.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub struct ConnectorConfig {
    #[cfg_attr(feature = "serde", serde(default))]
    pub solana: Solana,
    #[cfg_attr(feature = "serde", serde(default))]
    pub synchronizer: Synchronizer,
}

/// Solana network connection settings.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub struct Solana {
    pub rpc_url: String,
    pub ws_url: String,
    #[cfg_attr(feature = "serde", serde(with = "serde_commitment"))]
    pub commitment: CommitmentLevel,
}

/// Settings for the event synchronizer.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub struct Synchronizer {
    pub max_catchup_depth: Option<u64>,
    pub poll_interval_secs: u64,
    pub max_signature_fetch: usize,
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            solana: Solana::default(),
            synchronizer: Synchronizer::default(),
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

// Весь этот модуль нужен только для serde, поэтому оборачиваем его целиком
#[cfg(feature = "serde")]
mod serde_commitment {
    use super::*;
    // Этот use нужен внутри модуля, когда он активен
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
            _ => CommitmentLevel::Confirmed,
        };
        Ok(level)
    }
}
