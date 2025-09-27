mod catchup;
mod live;
mod synchronizer;

use crate::{
    config::Config, dispatcher::Dispatcher, events::BridgeEvent, storage::Storage,
    workers::synchronizer::Synchronizer,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, mpsc};

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

// A channel for sending new subscription requests to the running Dispatcher.
type RegistrationTx = mpsc::Sender<(Pubkey, mpsc::Sender<BridgeEvent>)>;

/// The main library client, managing the connection to Solana and providing targeted event streams.
pub struct EventWorker {
    registration_tx: RegistrationTx,
}

impl EventWorker {
    /// Creates a new EventWorker and starts the background services.
    pub fn new(
        config: Arc<Config>,
        storage: Arc<dyn Storage>, // This uses your Storage trait
        broadcast_capacity: usize,
        registration_capacity: usize,
    ) -> Self {
        // This is the main broadcast channel.
        // Your Synchronizer will send ALL events here.
        let (event_tx, event_rx) = broadcast::channel(broadcast_capacity);

        // This channel is for sending registration requests TO the dispatcher.
        let (reg_tx, reg_rx) = mpsc::channel(registration_capacity);

        // We create the Dispatcher using the updated `new` function.
        // It starts with zero listeners.
        let mut dispatcher = Dispatcher::new(event_rx, HashMap::new(), reg_rx);
        tokio::spawn(async move {
            dispatcher.run().await;
        });

        // We start your Synchronizer. Its API does NOT need to change.
        Synchronizer::start(config, storage, event_tx);

        tracing::info!("EventWorker initialized and background services are running.");

        Self {
            registration_tx: reg_tx,
        }
    }

    /// Dynamically subscribes to events for a given public key.
    pub async fn subscribe(
        &self,
        pubkey: Pubkey,
        channel_capacity: usize,
    ) -> mpsc::Receiver<BridgeEvent> {
        let (tx, rx) = mpsc::channel(channel_capacity);

        // This sends the subscription request to the running Dispatcher task.
        self.registration_tx
            .send((pubkey, tx))
            .await
            .expect("Dispatcher should always be running");

        rx
    }
}
