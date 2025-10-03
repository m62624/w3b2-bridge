use anyhow::Result;
use async_trait::async_trait;

/// A trait defining the required functionality for a persistent storage backend.
/// This allows for different database implementations.
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
