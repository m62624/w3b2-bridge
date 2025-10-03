//! # Contextual Event Listeners
//!
//! This module provides high-level, contextual event listeners (`UserListener`, `AdminListener`)
//! that abstract away the raw, unified event stream from the `Dispatcher`. Instead of a
//! single channel of undifferentiated events, these listeners categorize events based on the
//! role and context of the `ChainCard` being monitored.
//!
//! ## Philosophy
//!
//! The core idea is to move the filtering and categorization logic into the library itself,
//! providing the end-user with clean, purpose-driven event streams. This prevents the user's
//! application code from needing a complex `match` statement to handle every possible event type
//! and allows them to focus on the business logic for specific scenarios.
//!
//! ## Listener Types
//!
//! ### `UserListener`
//! Monitors events from the perspective of an end-user's `ChainCard`. It separates events into
//! three distinct categories:
//!
//! - **`personal_events`**: A stream for "solo" actions initiated by the user that do not
//!   directly involve an admin in the transaction. This includes managing their funds and profile.
//!   - Contains: `UserFundsDeposited`, `UserFundsWithdrawn`, `UserCommKeyUpdated`, `UserProfileClosed`, `OffChainActionLogged`.
//!
//! - **`all_service_interactions`**: A "discovery" stream that captures *all* events where the user
//!   interacts with *any* service/admin. Its primary purpose is to detect `UserProfileCreated`
//!   events, signaling that the user has established a new relationship with a service.
//!   - Contains: `UserProfileCreated`, `UserCommandDispatched`, `AdminCommandDispatched`.
//!
//! - **`listen_for_service(admin_pubkey)`**: A method to create a *targeted* stream for a single,
//!   specific user-service relationship. Once a service relationship is discovered via the
//!   `all_service_interactions` stream, this method can be used to listen for events
//!   (like `UserCommandDispatched`) related *only* to that specific admin.
//!
//! ### `AdminListener`
//! Monitors events from the perspective of a service provider's `ChainCard`. It provides
//! streams tailored to the operational needs of a service.
//!
//! - **`personal_events`**: A stream for actions the admin performs on their own `AdminProfile`.
//!   - Contains: `AdminProfileRegistered`, `AdminPricesUpdated`, `AdminFundsWithdrawn`, `AdminCommKeyUpdated`, `AdminProfileClosed`, `AdminCommandDispatched`, `OffChainActionLogged`.
//!
//! - **`new_user_profiles`**: The "discovery" stream for an admin. It emits an event only when a new
//!   user creates a `UserProfile` for this admin's service. This acts as a "doorbell" for new customers.
//!   - Contains: `UserProfileCreated`.
//!
//! - **`incoming_user_commands`**: The primary operational stream for a service, delivering all
//!   commands sent by users to this specific admin.
//!   - Contains: `UserCommandDispatched`.

use crate::events::BridgeEvent;
use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use tokio::sync::mpsc;
use w3b2_bridge_program::ID as PROGRAM_ID;

// --- User Listener ---

/// Manages event streams from a user's perspective.
///
/// A `UserListener` categorizes raw events into three distinct
/// channels for easier consumption in application code:
///
/// - **personal events**: Self-initiated user actions (deposits, withdrawals, profile changes).
/// - **all service interactions**: Events capturing *any* user ↔ service interaction.
/// - **service-specific streams**: Dynamically created channels to isolate interactions with a
///   single service/admin.
pub struct UserListener {
    /// Channel for personal user events.
    personal_events_rx: mpsc::Receiver<BridgeEvent>,
    /// Channel for all service-related interactions.
    all_interactions_rx: mpsc::Receiver<BridgeEvent>,
    /// Map of service-specific listeners keyed by `Admin PDA`.
    service_listeners: Arc<DashMap<Pubkey, mpsc::Sender<BridgeEvent>>>,
}

impl UserListener {
    /// Create a new `UserListener`.
    ///
    /// - `pubkey`: The authority public key of the user.
    /// - `raw_event_rx`: The unified event stream produced by the dispatcher.
    /// - `channel_capacity`: Capacity for each internal mpsc channel.
    ///
    /// Spawns a background task that routes events into the categorized channels.
    pub fn new(
        pubkey: Pubkey,
        mut raw_event_rx: mpsc::Receiver<BridgeEvent>,
        channel_capacity: usize,
    ) -> Self {
        let (personal_tx, personal_rx) = mpsc::channel(channel_capacity);
        let (all_interactions_tx, all_interactions_rx) = mpsc::channel(channel_capacity);
        let service_listeners = Arc::new(DashMap::new());
        let service_listeners_clone = service_listeners.clone();

        tokio::spawn(async move {
            while let Some(event) = raw_event_rx.recv().await {
                match &event {
                    // --- Personal Events ---
                    BridgeEvent::UserFundsDeposited(e) if e.authority == pubkey => {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::UserFundsWithdrawn(e) if e.authority == pubkey => {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::UserCommKeyUpdated(e) if e.authority == pubkey => {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::UserProfileClosed(e) if e.authority == pubkey => {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::OffChainActionLogged(e) if e.actor == pubkey => {
                        let _ = personal_tx.send(event).await;
                    }

                    // --- Interaction Events ---
                    BridgeEvent::UserProfileCreated(e) if e.authority == pubkey => {
                        handle_interaction(event, &all_interactions_tx, &service_listeners_clone)
                            .await;
                    }
                    BridgeEvent::UserCommandDispatched(e) if e.sender == pubkey => {
                        handle_interaction(event, &all_interactions_tx, &service_listeners_clone)
                            .await;
                    }
                    BridgeEvent::AdminCommandDispatched(e) if e.target_user_authority == pubkey => {
                        handle_interaction(event, &all_interactions_tx, &service_listeners_clone)
                            .await;
                    }
                    _ => {}
                }
            }
        });

        Self {
            personal_events_rx: personal_rx,
            all_interactions_rx,
            service_listeners,
        }
    }

    /// Access the channel of **personal user events**.
    ///
    /// Events include deposits, withdrawals, comm key updates, and profile closure.
    pub fn personal_events(&mut self) -> &mut mpsc::Receiver<BridgeEvent> {
        &mut self.personal_events_rx
    }

    /// Access the channel of **all service interactions**.
    ///
    /// Events include any user ↔ admin relationship creation or command dispatch.
    pub fn all_service_interactions(&mut self) -> &mut mpsc::Receiver<BridgeEvent> {
        &mut self.all_interactions_rx
    }

    /// Create a new channel for events tied to a **specific service/admin**.
    ///
    /// - `target_admin_pda`: The PDA of the target service/admin.
    /// - `capacity`: Channel buffer capacity.
    ///
    /// Returns a `Receiver` that emits only events related to this admin.
    pub fn listen_for_service(
        &self,
        target_admin_pda: Pubkey,
        capacity: usize,
    ) -> mpsc::Receiver<BridgeEvent> {
        let (tx, rx) = mpsc::channel(capacity);
        self.service_listeners.insert(target_admin_pda, tx);
        rx
    }

    /// Consumes the listener and returns its underlying receiver channels.
    /// This is useful for moving the channels into separate tasks, like in `tokio::select!`.
    pub fn into_parts(self) -> (mpsc::Receiver<BridgeEvent>, mpsc::Receiver<BridgeEvent>) {
        (self.personal_events_rx, self.all_interactions_rx)
    }
}

// --- Admin Listener ---

/// Manages event streams from an admin/service perspective.
///
/// An `AdminListener` categorizes raw events into three distinct
/// channels tailored for service-provider logic:
///
/// - **personal events**: Admin self-initiated actions.
/// - **new user profiles**: Discovery of new customers.
/// - **incoming user commands**: Operational stream of requests from users.
pub struct AdminListener {
    /// Channel for admin-only events.
    personal_events_rx: mpsc::Receiver<BridgeEvent>,
    /// Channel for incoming user commands targeted to this admin.
    incoming_user_commands_rx: mpsc::Receiver<BridgeEvent>,
    /// Channel for new user profile creation events.
    new_user_profiles_rx: mpsc::Receiver<BridgeEvent>,
}

impl AdminListener {
    /// Create a new `AdminListener`.
    ///
    /// - `admin_authority_pubkey`: The admin's authority pubkey.
    /// - `raw_event_rx`: The unified event stream from the dispatcher.
    /// - `channel_capacity`: Capacity for each internal mpsc channel.
    ///
    /// Spawns a background task that routes events into the categorized channels.
    pub fn new(
        admin_authority_pubkey: Pubkey,
        mut raw_event_rx: mpsc::Receiver<BridgeEvent>,
        channel_capacity: usize,
    ) -> Self {
        let (personal_tx, personal_rx) = mpsc::channel(channel_capacity);
        let (commands_tx, commands_rx) = mpsc::channel(channel_capacity);
        let (new_users_tx, new_users_rx) = mpsc::channel(channel_capacity);

        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", admin_authority_pubkey.as_ref()], &PROGRAM_ID);

        tokio::spawn(async move {
            while let Some(event) = raw_event_rx.recv().await {
                match &event {
                    // --- Personal Admin Events ---
                    BridgeEvent::AdminProfileRegistered(e)
                        if e.authority == admin_authority_pubkey =>
                    {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::AdminPricesUpdated(e) if e.authority == admin_authority_pubkey => {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::AdminFundsWithdrawn(e)
                        if e.authority == admin_authority_pubkey =>
                    {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::AdminCommKeyUpdated(e)
                        if e.authority == admin_authority_pubkey =>
                    {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::AdminProfileClosed(e) if e.authority == admin_authority_pubkey => {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::AdminCommandDispatched(e)
                        if e.sender == admin_authority_pubkey =>
                    {
                        let _ = personal_tx.send(event).await;
                    }
                    BridgeEvent::OffChainActionLogged(e) if e.actor == admin_authority_pubkey => {
                        let _ = personal_tx.send(event).await;
                    }

                    // --- User → Admin Events ---
                    BridgeEvent::UserCommandDispatched(e)
                        if e.target_admin_authority == admin_authority_pubkey =>
                    {
                        let _ = commands_tx.send(event).await;
                    }
                    BridgeEvent::UserProfileCreated(e) if e.target_admin == admin_pda => {
                        let _ = new_users_tx.send(event).await;
                    }
                    _ => {}
                }
            }
        });

        Self {
            personal_events_rx: personal_rx,
            incoming_user_commands_rx: commands_rx,
            new_user_profiles_rx: new_users_rx,
        }
    }

    /// Access the channel of **personal admin events**.
    ///
    /// Includes profile registration, price updates, withdrawals,
    /// comm key updates, and profile closure.
    pub fn personal_events(&mut self) -> &mut mpsc::Receiver<BridgeEvent> {
        &mut self.personal_events_rx
    }

    /// Access the channel of **incoming user commands**.
    ///
    /// Provides the operational command stream for this admin's service.
    pub fn incoming_user_commands(&mut self) -> &mut mpsc::Receiver<BridgeEvent> {
        &mut self.incoming_user_commands_rx
    }

    /// Access the channel of **new user profiles**.
    ///
    /// Emits events when a new user creates a profile for this admin.
    pub fn new_user_profiles(&mut self) -> &mut mpsc::Receiver<BridgeEvent> {
        &mut self.new_user_profiles_rx
    }

    /// Consumes the listener and returns its underlying receiver channels.
    /// This is useful for moving the channels into separate tasks, like in `tokio::select!`.
    pub fn into_parts(
        self,
    ) -> (
        mpsc::Receiver<BridgeEvent>,
        mpsc::Receiver<BridgeEvent>,
        mpsc::Receiver<BridgeEvent>,
    ) {
        (
            self.personal_events_rx,
            self.incoming_user_commands_rx,
            self.new_user_profiles_rx,
        )
    }
}

// --- Helper functions ---

/// Process a user interaction event for a `UserListener`.
///
/// Routes the event into the **all service interactions** channel,
/// and, if a matching admin-specific listener exists,
/// into the appropriate service-specific channel as well.
async fn handle_interaction(
    event: BridgeEvent,
    all_interactions_tx: &mpsc::Sender<BridgeEvent>,
    service_listeners: &Arc<DashMap<Pubkey, mpsc::Sender<BridgeEvent>>>,
) {
    if all_interactions_tx.send(event.clone()).await.is_err() {
        return;
    }
    if let Some(admin_pubkey) = get_admin_pubkey_from_interaction(&event) {
        if let Some(specific_tx) = service_listeners.get(&admin_pubkey) {
            let _ = specific_tx.send(event).await;
        }
    }
}

/// Extract the relevant admin pubkey (authority or PDA) from an interaction event.
///
/// Returns `Some(pubkey)` if the event type contains an admin reference,
/// otherwise returns `None`.
fn get_admin_pubkey_from_interaction(event: &BridgeEvent) -> Option<Pubkey> {
    match event {
        BridgeEvent::UserProfileCreated(e) => Some(e.target_admin),
        BridgeEvent::UserCommandDispatched(e) => Some(e.target_admin_authority),
        BridgeEvent::AdminCommandDispatched(e) => Some(e.sender),
        _ => None,
    }
}
