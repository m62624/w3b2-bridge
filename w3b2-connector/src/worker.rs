use crate::{config::Config, events::BridgeEvent, storage::Storage};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use tokio::sync::broadcast;

/// A shared context containing all dependencies required by the workers.
#[derive(Clone)]
pub struct WorkerContext {
    pub config: Arc<Config>,
    pub storage: Arc<dyn Storage>,
    pub rpc_client: Arc<RpcClient>,
    pub event_sender: broadcast::Sender<BridgeEvent>,
}

impl WorkerContext {
    pub fn new(
        config: Arc<Config>,
        storage: Arc<dyn Storage>,
        event_sender: broadcast::Sender<BridgeEvent>,
    ) -> Self {
        let rpc_client = Arc::new(RpcClient::new(config.solana.rpc_url.clone()));
        Self {
            config,
            storage,
            rpc_client,
            event_sender,
        }
    }
}
