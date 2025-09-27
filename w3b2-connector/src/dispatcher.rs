// w3b2-connector/src/dispatcher.rs

use crate::events::BridgeEvent;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc};

/// The Dispatcher is responsible for receiving all events from the Synchronizer
/// and routing them to the appropriate ChainCard worker based on the public keys
/// involved in the event.
pub struct Dispatcher {
    /// Receives all events from the Synchronizer's broadcast channel.
    event_rx: broadcast::Receiver<BridgeEvent>,
    /// Maps a ChainCard's public key to the sender half of its dedicated channel.
    worker_channels: HashMap<Pubkey, mpsc::Sender<BridgeEvent>>,
}

impl Dispatcher {
    pub fn new(
        event_rx: broadcast::Receiver<BridgeEvent>,
        worker_channels: HashMap<Pubkey, mpsc::Sender<BridgeEvent>>,
    ) -> Self {
        Self {
            event_rx,
            worker_channels,
        }
    }

    /// Starts the main event-loop for the dispatcher.
    pub async fn run(&mut self) {
        tracing::info!("Dispatcher started. Waiting for events...");
        loop {
            match self.event_rx.recv().await {
                Ok(event) => {
                    let relevant_pubkeys = extract_pubkeys_from_event(&event);
                    for pubkey in relevant_pubkeys {
                        if let Some(worker_tx) = self.worker_channels.get(&pubkey) {
                            if worker_tx.send(event.clone()).await.is_err() {
                                tracing::warn!("Worker channel for pubkey {} is closed.", pubkey);
                            }
                        }
                    }
                }
                // Case 1: The receiver is lagging and skipped some messages.
                Err(broadcast::error::RecvError::Lagged(skipped_count)) => {
                    tracing::warn!("Dispatcher lagged, skipped {} messages.", skipped_count);
                    // We continue the loop, as the channel is still open.
                    continue;
                }
                // Case 2: The sender has been dropped, so no more messages will arrive.
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::error!("Broadcast channel closed. Dispatcher shutting down.");
                    // We break the loop to stop the task.
                    break;
                }
            }
        }
    }
}

/// Helper function to extract all relevant public keys from an event.
/// An event is relevant to a ChainCard if its public key is mentioned as a
/// sender, receiver, authority, etc.
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
