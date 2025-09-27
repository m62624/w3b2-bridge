// w3b2-connector/src/synchronizer.rs

use crate::{
    config::Config,
    events::BridgeEvent,
    storage::Storage,
    workers::{catchup::CatchupWorker, live::LiveWorker, WorkerContext},
};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct Synchronizer;

impl Synchronizer {
    /// Creates and runs both the catch-up and live workers in the background.
    pub fn start(
        config: Arc<Config>,
        storage: Arc<dyn Storage>,
        event_tx: broadcast::Sender<BridgeEvent>,
    ) {
        let context = WorkerContext::new(config, storage, event_tx);

        // Run the catch-up worker
        let catchup_worker = CatchupWorker::new(context.clone());
        tokio::spawn(async move {
            if let Err(e) = catchup_worker.run().await {
                tracing::error!("Catch-up worker failed: {}", e);
            }
        });

        // Run the live worker
        let live_worker = LiveWorker::new(context);
        tokio::spawn(async move {
            if let Err(e) = live_worker.run().await {
                tracing::error!("Live worker failed: {}", e);
            }
        });
    }
}
