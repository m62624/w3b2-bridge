/// Provides concrete `sled`-based implementations for the storage traits
/// defined in the `w3b2-connector` library.
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use sled::{Db, transaction::TransactionalTree};

use w3b2_connector::storage::Storage;

/// A `sled`-backed implementation of the `Storage` trait.
///
/// It uses a single `sled` database to transactionally store the `last_slot`
/// and `last_sig` processed by the synchronizer.
#[derive(Clone)]
pub struct SledStorage {
    db: Db,
}

impl SledStorage {
    /// Creates a new instance of `SledStorage`.
    ///
    /// # Arguments
    ///
    /// * `db` - A `sled::Db` instance. This can be shared with `SledKeystore`.
    pub fn new(db: Db) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Storage for SledStorage {
    /// Retrieves the last synchronized slot number from the database.
    /// Returns 0 if no slot has been stored yet.
    async fn get_last_slot(&self) -> Result<u64> {
        let result = self
            .db
            .get("sync::last_slot")?
            .and_then(|v| String::from_utf8(v.to_vec()).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        Ok(result)
    }

    /// Retrieves the last synchronized signature from the database.
    /// Returns `None` if no signature has been stored yet.
    async fn get_last_sig(&self) -> Result<Option<String>> {
        let result = self
            .db
            .get("sync::last_sig")?
            .and_then(|v| String::from_utf8(v.to_vec()).ok());
        Ok(result)
    }

    /// Atomically sets the last synchronized slot and signature using a `sled` transaction.
    /// This ensures that the sync state is always consistent.
    async fn set_sync_state(&self, slot: u64, sig: &str) -> Result<()> {
        self.db.transaction(
            |tx: &TransactionalTree| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                tx.insert("sync::last_slot", slot.to_string().as_bytes())?;
                tx.insert("sync::last_sig", sig.as_bytes())?;
                Ok(())
            },
        ).map_err(|e| anyhow!("Sled transaction for sync state failed: {:?}", e))?;

        self.db.flush_async().await?;

        Ok(())
    }
}
