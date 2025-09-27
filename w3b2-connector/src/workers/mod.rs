mod catchup;
mod live;
mod synchronizer;

use crate::{
    config::ConnectorConfig,
    dispatcher::Dispatcher,
    events::BridgeEvent,
    listener::{AdminListener, UserListener},
    storage::Storage,
    workers::synchronizer::Synchronizer,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, mpsc};

/// A shared context containing all dependencies required by the workers.
#[derive(Clone)]
struct WorkerContext {
    pub config: Arc<ConnectorConfig>,
    pub storage: Arc<dyn Storage>,
    pub rpc_client: Arc<RpcClient>,
    pub event_sender: broadcast::Sender<BridgeEvent>,
}

impl WorkerContext {
    fn new(
        config: Arc<ConnectorConfig>,
        rpc_client: Arc<RpcClient>,
        storage: Arc<dyn Storage>,
        event_sender: broadcast::Sender<BridgeEvent>,
    ) -> Self {
        Self {
            config,
            storage,
            rpc_client,
            event_sender,
        }
    }
}

/// The main library client, which manages the connection to Solana and provides
/// high-level, contextual event listeners. This is the primary entry point for users
/// of the library.
pub struct EventManager {
    synchronizer: Synchronizer,
    dispatcher: Dispatcher,
    pub registration_tx: mpsc::Sender<(Pubkey, mpsc::Sender<BridgeEvent>)>,
}

impl EventManager {
    pub fn new(
        config: Arc<ConnectorConfig>,
        rpc_client: Arc<RpcClient>,
        storage: Arc<dyn Storage>,
        // Capacities are now arguments for better control by the binary.
        broadcast_capacity: usize,
        registration_capacity: usize,
    ) -> Self {
        let (event_tx, event_rx) = broadcast::channel(broadcast_capacity);
        let (reg_tx, reg_rx) = mpsc::channel(registration_capacity);

        let synchronizer = Synchronizer::new(
            config.clone(),
            rpc_client.clone(),
            storage.clone(),
            event_tx,
        );

        let dispatcher = Dispatcher::new(event_rx, HashMap::new(), reg_rx);

        Self {
            synchronizer,
            dispatcher,
            registration_tx: reg_tx,
        }
    }

    /// Runs all background services of the connector.
    /// This method should be spawned as a background task by the application.
    pub async fn run(mut self) {
        tracing::info!("Connector is running all background services.");
        // We can run them in a select loop to shut down if one of them fails.
        tokio::select! {
            _ = self.synchronizer.run() => {
                tracing::error!("Synchronizer exited unexpectedly.");
            },
            _ = self.dispatcher.run() => {
                tracing::error!("Dispatcher exited unexpectedly.");
            }
        }
    }

    /// (Internal) Creates a raw, un-filtered subscription for a pubkey.
    /// This is the low-level building block for the high-level listeners.
    async fn subscribe_raw(
        &self,
        pubkey: Pubkey,
        channel_capacity: usize,
    ) -> mpsc::Receiver<BridgeEvent> {
        let (tx, rx) = mpsc::channel(channel_capacity);
        self.registration_tx
            .send((pubkey, tx))
            .await
            .expect("Dispatcher should always be running");
        rx
    }

    /// Creates and returns a contextual listener for a User `ChainCard`.
    /// This is the primary method for users of the library to listen to their events.
    ///
    /// * `user_pubkey` - The public key of the user's `ChainCard` to monitor.
    /// * `channel_capacity` - The buffer capacity for the internal event channels.
    pub async fn listen_as_user(
        &self,
        user_pubkey: Pubkey,
        channel_capacity: usize,
    ) -> UserListener {
        // 1. Get the raw event stream for the user's pubkey.
        let raw_rx = self.subscribe_raw(user_pubkey, channel_capacity).await;
        // 2. Construct the high-level listener that will consume and categorize the raw stream.
        UserListener::new(user_pubkey, raw_rx, channel_capacity)
    }

    /// Creates and returns a contextual listener for an Admin `ChainCard`.
    ///
    /// * `admin_pubkey` - The public key of the admin's `ChainCard` to monitor.
    /// * `channel_capacity` - The buffer capacity for the internal event channels.
    pub async fn listen_as_admin(
        &self,
        admin_pubkey: Pubkey,
        channel_capacity: usize,
    ) -> AdminListener {
        // 1. Get the raw event stream for the admin's pubkey.
        let raw_rx = self.subscribe_raw(admin_pubkey, channel_capacity).await;
        // 2. Construct the high-level listener.
        AdminListener::new(admin_pubkey, raw_rx, channel_capacity)
    }
}
