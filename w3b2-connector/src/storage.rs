// w3b2-connector/src/storage/mod.rs

use anyhow::{Context, Result};
use async_trait::async_trait;
use sled::transaction::TransactionalTree;
use sled::Db;

/// A trait defining the required functionality for a persistent storage backend.
/// This allows for different database implementations (e.g., Sled, Postgres).
#[async_trait]
pub trait Storage: Send + Sync {
    /// Retrieves the last synchronized slot number from the storage.
    async fn get_last_slot(&self) -> Result<u64>;

    /// Retrieves the last synchronized signature from the storage.
    async fn get_last_sig(&self) -> Result<Option<String>>;

    /// Atomically sets the last synchronized slot and signature.
    /// This should be a transactional operation to ensure data consistency.
    async fn set_sync_state(&self, slot: u64, sig: &str) -> Result<()>;
}

#[derive(Clone)]
pub struct SledStorage {
    db: Db,
}

impl SledStorage {
    pub fn new(path: &str) -> Result<Self> {
        Ok(Self {
            db: sled::open(path).context("Failed to open Sled database")?,
        })
    }
}

#[async_trait]
impl Storage for SledStorage {
    async fn get_last_slot(&self) -> Result<u64> {
        let result = self
            .db
            .get("last_slot")?
            .and_then(|v| String::from_utf8(v.to_vec()).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(result)
    }

    async fn get_last_sig(&self) -> Result<Option<String>> {
        let result = self
            .db
            .get("last_sig")?
            .and_then(|v| String::from_utf8(v.to_vec()).ok());
        Ok(result)
    }

    async fn set_sync_state(&self, slot: u64, sig: &str) -> Result<()> {
        self.db.transaction(
            |tx: &TransactionalTree| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                tx.insert("last_slot", slot.to_string().as_bytes())?;
                tx.insert("last_sig", sig.as_bytes())?;
                Ok(())
            },
        ).map_err(|e| anyhow::anyhow!("Sled transaction failed: {:?}", e))?;

        self.db.flush_async().await?;

        Ok(())
    }
}
