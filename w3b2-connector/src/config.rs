use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::CommitmentLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// RPC HTTP endpoint
    pub rpc_url: String,
    /// RPC WS endpoint
    pub ws_url: String,
    /// Max slots to go back during catch-up. `None` means unlimited.
    pub max_catchup_depth: Option<u64>,
    /// Max age of a funding request in minutes to be processed
    pub max_request_age_minutes: u64,
    #[serde(default = "default_time_provider", with = "serde_rfc3339")]
    pub time_provider: DateTime<Utc>,
    /// Poll interval in seconds for catch-up worker
    /// Default: 3 seconds
    pub poll_interval_secs: Option<u64>,
    /// Commitment level for RPC requests
    /// Default: Confirmed
    #[serde(with = "serde_commitment")]
    pub commitment: Option<solana_sdk::commitment_config::CommitmentLevel>,
    /// Max number of signatures to fetch in one RPC call
    pub max_signature_fetch: Option<usize>,
    pub data_dir: String,
    pub log_dir: String,
    /// gRPC server host
    #[serde(default = "default_host")]
    pub host: String,
    /// gRPC server port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Logging format: "plain" | "json"
    #[serde(default = "default_log_format")]
    pub log_format: String,
    /// Logging level: "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR"
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_time_provider() -> DateTime<Utc> {
    Utc::now()
}

fn default_host() -> String {
    "[::1]".to_string()
}

fn default_port() -> u16 {
    50051
}

fn default_log_format() -> String {
    "plain".to_string()
}

fn default_log_level() -> String {
    "INFO".to_string()
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8899".into(),
            ws_url: "ws://127.0.0.1:8900".into(),
            max_catchup_depth: None,
            max_request_age_minutes: 60, // 1 час
            time_provider: Utc::now(),
            poll_interval_secs: Some(3),
            commitment: Some(CommitmentLevel::Confirmed),
            max_signature_fetch: None,
            data_dir: "./w3b2_db".into(),
            log_dir: "Logs".into(),
            host: default_host(),
            port: default_port(),
            log_format: default_log_format(),
            log_level: default_log_level(),
        }
    }
}

mod serde_commitment {
    use serde::{Deserializer, Serializer};

    use super::*;

    pub fn serialize<S>(c: &Option<CommitmentLevel>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match c {
            Some(CommitmentLevel::Processed) => "Processed",
            Some(CommitmentLevel::Confirmed) => "Confirmed",
            Some(CommitmentLevel::Finalized) => "Finalized",
            None => "Confirmed",
        };
        serializer.serialize_str(s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<CommitmentLevel>, D::Error>
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
        Ok(Some(level))
    }
}

mod serde_rfc3339 {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.to_rfc3339())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<DateTime<Utc>>().map_err(serde::de::Error::custom)
    }
}
