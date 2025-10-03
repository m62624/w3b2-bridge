/// # Event Dispatcher
///
/// The `Dispatcher` is a background worker that routes on-chain events from the global
/// "firehose" stream into clean, filtered streams for specific listeners.
///
/// ## What Problem It Solves
/// Instead of every `UserListener` or `AdminListener` scanning through thousands of
/// irrelevant events, the `Dispatcher` inspects each incoming event and forwards it only
/// to the listeners that care about the involved public keys.
///
/// ## Why Separate
/// The `Dispatcher` runs continuously in the background, maintaining subscriptions and
/// routing logic. This separation keeps the public-facing `EventManager` simple and
/// allows safe, concurrent event handling.
///
/// ## Extensibility
/// Any other service (e.g. gRPC streaming, audit logging) can hook into the raw broadcast
/// channel from the `Synchronizer`, bypassing the dispatcher entirely if unfiltered access
/// is needed.
use crate::events::BridgeEvent;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc};

/// The Dispatcher is responsible for receiving all events from the Synchronizer
/// and routing them to the appropriate listeners based on the public keys
/// involved in the event.
pub struct Dispatcher {
    // This receives all events from the Synchronizer's broadcast channel.
    event_rx: broadcast::Receiver<BridgeEvent>,
    // This stores the dedicated channels for listeners who have subscribed.
    listeners: HashMap<Pubkey, mpsc::Sender<BridgeEvent>>,
    // This channel now receives commands, not just registrations.
    command_rx: mpsc::Receiver<DispatcherCommand>,
}

/// Defines commands that can be sent to the Dispatcher task.
#[derive(Debug)]
pub enum DispatcherCommand {
    /// Registers a new listener for a given public key.
    Register(Pubkey, mpsc::Sender<BridgeEvent>),
    /// Unregisters a listener for a given public key.
    Unregister(Pubkey),
}

impl Dispatcher {
    pub fn new(
        event_rx: broadcast::Receiver<BridgeEvent>,
        command_rx: mpsc::Receiver<DispatcherCommand>,
    ) -> Self {
        Self {
            event_rx,
            listeners: HashMap::new(),
            command_rx,
        }
    }

    /// Starts the main event-loop for the dispatcher.
    pub async fn run(&mut self) {
        tracing::info!("Dispatcher started. Waiting for events and commands...");
        loop {
            tokio::select! {
                // An event arrived from the blockchain.
                Ok(event) = self.event_rx.recv() => {
                    let relevant_pubkeys = extract_pubkeys_from_event(&event);
                    for pubkey in relevant_pubkeys {
                        if let Some(listener_tx) = self.listeners.get(&pubkey) {
                            if listener_tx.send(event.clone()).await.is_err() {
                                // The receiver was dropped. The active `unsubscribe` call will clean this up,
                                // but logging it is still useful.
                                tracing::warn!("Attempted to send to a disconnected listener for pubkey {}.", pubkey);
                            }
                        }
                    }
                },
                // A command to register or unregister a listener arrived.
                Some(command) = self.command_rx.recv() => {
                    match command {
                        DispatcherCommand::Register(pubkey, tx) => {
                            tracing::info!("Dispatcher: Registering new listener for {}", pubkey);
                            self.listeners.insert(pubkey, tx);
                        },
                        DispatcherCommand::Unregister(pubkey) => {
                            tracing::info!("Dispatcher: Unregistering listener for {}", pubkey);
                            self.listeners.remove(&pubkey);
                        }
                    }
                },
                else => {
                    tracing::error!("All channels closed. Dispatcher shutting down.");
                    break;
                }
            }
        }
    }
}

/// Helper function to extract all relevant public keys from an event.
fn extract_pubkeys_from_event(event: &BridgeEvent) -> Vec<Pubkey> {
    use w3b2_bridge_program::events as OnChainEvent;
    match event {
        BridgeEvent::AdminProfileRegistered(OnChainEvent::AdminProfileRegistered {
            authority,
            ..
        }) => vec![*authority],
        BridgeEvent::AdminCommKeyUpdated(OnChainEvent::AdminCommKeyUpdated {
            authority, ..
        }) => vec![*authority],
        BridgeEvent::AdminPricesUpdated(OnChainEvent::AdminPricesUpdated { authority, .. }) => {
            vec![*authority]
        }
        BridgeEvent::AdminFundsWithdrawn(OnChainEvent::AdminFundsWithdrawn {
            authority, ..
        }) => vec![*authority],
        BridgeEvent::AdminProfileClosed(OnChainEvent::AdminProfileClosed { authority, .. }) => {
            vec![*authority]
        }
        BridgeEvent::UserProfileCreated(OnChainEvent::UserProfileCreated {
            authority,
            target_admin,
            ..
        }) => vec![*authority, *target_admin],
        BridgeEvent::UserCommKeyUpdated(OnChainEvent::UserCommKeyUpdated { authority, .. }) => {
            vec![*authority]
        }
        BridgeEvent::UserFundsDeposited(OnChainEvent::UserFundsDeposited { authority, .. }) => {
            vec![*authority]
        }
        BridgeEvent::UserFundsWithdrawn(OnChainEvent::UserFundsWithdrawn { authority, .. }) => {
            vec![*authority]
        }
        BridgeEvent::UserProfileClosed(OnChainEvent::UserProfileClosed { authority, .. }) => {
            vec![*authority]
        }
        BridgeEvent::UserCommandDispatched(OnChainEvent::UserCommandDispatched {
            sender,
            target_admin_authority,
            ..
        }) => vec![*sender, *target_admin_authority],
        BridgeEvent::AdminCommandDispatched(OnChainEvent::AdminCommandDispatched {
            sender,
            target_user_authority,
            ..
        }) => vec![*sender, *target_user_authority],
        BridgeEvent::OffChainActionLogged(OnChainEvent::OffChainActionLogged { actor, .. }) => {
            vec![*actor]
        }
        BridgeEvent::Unknown => vec![],
    }
}
