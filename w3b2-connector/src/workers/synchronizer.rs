use crate::{
    config::ConnectorConfig,
    events::BridgeEvent,
    storage::Storage,
    workers::{catchup::CatchupWorker, live::LiveWorker, WorkerContext},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct Synchronizer {
    catchup_worker: CatchupWorker,
    live_worker: LiveWorker,
}

impl Synchronizer {
    /// Creates a new `Synchronizer` instance, preparing the workers but not starting them.
    pub fn new(
        config: Arc<ConnectorConfig>,
        rpc_client: Arc<RpcClient>,
        storage: Arc<dyn Storage>,
        event_tx: broadcast::Sender<BridgeEvent>,
    ) -> Self {
        let context = WorkerContext::new(config, rpc_client, storage, event_tx);
        let catchup_worker = CatchupWorker::new(context.clone());
        let live_worker = LiveWorker::new(context);

        Self {
            catchup_worker,
            live_worker,
        }
    }

    /// Runs both the catch-up and live workers concurrently.
    ///
    /// This method will run indefinitely until one of the workers fails or the parent task is cancelled.
    /// This should be called and awaited by the application's main runtime.
    pub async fn run(self) -> anyhow::Result<()> {
        tracing::info!("Starting synchronizer workers...");

        // Run both workers concurrently. `tokio::try_join!` will return
        // immediately if any of the workers returns an error.
        tokio::try_join!(self.catchup_worker.run(), self.live_worker.run())?;

        Ok(())
    }
}
