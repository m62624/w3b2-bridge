use anchor_lang::prelude::*;

// --- Admin Events ---

/// Emitted when a new AdminProfile PDA is created.
/// This signifies that a new service has been registered on the protocol.
#[event]
#[derive(Debug)]
pub struct AdminProfileRegistered {
    /// The public key of the admin's `ChainCard`, which serves as the unique owner
    /// and signer for the `AdminProfile`.
    pub authority: Pubkey,
    /// The public key provided by the admin for secure off-chain communication,
    /// typically used for hybrid encryption.
    pub communication_pubkey: Pubkey,
    /// The Unix timestamp (in seconds) when the registration occurred.
    pub ts: i64,
}

/// Emitted when an admin updates their off-chain communication public key.
#[event]
#[derive(Debug)]
pub struct AdminCommKeyUpdated {
    /// The public key of the admin's `ChainCard` that authorized this update.
    pub authority: Pubkey,
    /// The new communication public key that has been set for the `AdminProfile`.
    pub new_comm_pubkey: Pubkey,
    /// The Unix timestamp of the update.
    pub ts: i64,
}

/// Emitted when an admin updates their service prices.
#[event]
#[derive(Debug)]
pub struct AdminPricesUpdated {
    /// The public key of the `AdminProfile`'s owner (`ChainCard`).
    pub authority: Pubkey,
    /// A vector of tuples `(command_id, price)` representing the new price list for the service.
    pub new_prices: Vec<(u64, u64)>,
    /// The Unix timestamp of the price update.
    pub ts: i64,
}

/// Emitted when an admin withdraws earned funds from their profile's internal balance.
#[event]
#[derive(Debug)]
pub struct AdminFundsWithdrawn {
    /// The `ChainCard` public key of the admin who initiated the withdrawal.
    pub authority: Pubkey,
    /// The amount of lamports withdrawn from the `AdminProfile`'s internal balance.
    pub amount: u64,
    /// The public key of the wallet that received the withdrawn funds.
    pub destination: Pubkey,
    /// The Unix timestamp of the withdrawal.
    pub ts: i64,
}

/// Emitted when an `AdminProfile` PDA is closed, effectively unregistering the service.
#[event]
#[derive(Debug)]
pub struct AdminProfileClosed {
    /// The `ChainCard` public key of the admin whose profile was closed.
    pub authority: Pubkey,
    /// The Unix timestamp of the account closure.
    pub ts: i64,
}

/// Emitted when an admin sends a command (notification) to a user.
#[event]
#[derive(Debug)]
pub struct AdminCommandDispatched {
    /// The public key of the admin's `ChainCard`, who is the initiator of this command.
    pub sender: Pubkey,
    /// The public key of the target user's `ChainCard`.
    pub target_user_authority: Pubkey,
    /// A `u64` identifier for the specific command or notification being sent.
    pub command_id: u64,
    /// An opaque byte array containing application-specific data for the command.
    pub payload: Vec<u8>,
    /// The Unix timestamp when the command was dispatched.
    pub ts: i64,
}

// --- User Lifecycle & Financial Events ---

/// Emitted when a new `UserProfile` PDA is created, linking a user to a specific admin.
#[event]
#[derive(Debug)]
pub struct UserProfileCreated {
    /// The public key of the user's `ChainCard`, which is the sole owner of this `UserProfile`.
    pub authority: Pubkey,
    /// The public key of the `AdminProfile` PDA that this `UserProfile` is associated with.
    pub target_admin: Pubkey,
    /// The public key provided by the user for secure off-chain communication.
    pub communication_pubkey: Pubkey,
    /// The Unix timestamp of the profile creation.
    pub ts: i64,
}

/// Emitted when a user updates their off-chain communication public key.
#[event]
#[derive(Debug)]
pub struct UserCommKeyUpdated {
    /// The `ChainCard` public key of the user who authorized this update.
    pub authority: Pubkey,
    /// The new communication public key for the `UserProfile`.
    pub new_comm_pubkey: Pubkey,
    /// The Unix timestamp of the update.
    pub ts: i64,
}

/// Emitted when a user deposits funds into their `UserProfile` to pay for services.
#[event]
#[derive(Debug)]
pub struct FundsDeposited {
    /// The public key of the user (`ChainCard`) who made the deposit.
    pub authority: Pubkey,
    /// The amount of lamports deposited into the `UserProfile`.
    pub amount: u64,
    /// The user's new total `deposit_balance` after this transaction.
    pub new_deposit_balance: u64,
    /// The Unix timestamp of the deposit.
    pub ts: i64,
}

/// Emitted when a user withdraws unspent funds from their `UserProfile`.
#[event]
#[derive(Debug)]
pub struct FundsWithdrawn {
    /// The public key of the user (`ChainCard`) who made the withdrawal.
    pub authority: Pubkey,
    /// The amount of lamports withdrawn from the `UserProfile`.
    pub amount: u64,
    /// The public key of the wallet that received the funds.
    pub destination: Pubkey,
    /// The user's new total `deposit_balance` after this transaction.
    pub new_deposit_balance: u64,
    /// The Unix timestamp of the withdrawal.
    pub ts: i64,
}

/// Emitted when a `UserProfile` PDA is closed.
#[event]
#[derive(Debug)]
pub struct UserProfileClosed {
    /// The `ChainCard` public key of the user whose profile was closed.
    pub authority: Pubkey,
    /// The Unix timestamp of the account closure.
    pub ts: i64,
}

// --- Operational Events ---

/// Emitted when a user calls a service's command, potentially a paid one.
#[event]
#[derive(Debug)]
pub struct UserCommandDispatched {
    /// The public key of the user's `ChainCard`, who is the initiator of the command.
    pub sender: Pubkey,
    /// The public key of the admin's `ChainCard` that owns the target service.
    pub target_admin_authority: Pubkey,
    /// A `u64` identifier for the specific command being executed.
    pub command_id: u64,
    /// The amount in lamports deducted from the user's deposit balance for this command (0 if free).
    pub price_paid: u64,
    /// An opaque byte array containing application-specific data for the command.
    pub payload: Vec<u8>,
    /// The Unix timestamp when the command was dispatched.
    pub ts: i64,
}

/// A generic event for logging significant off-chain actions for auditing purposes.
#[event]
#[derive(Debug)]
pub struct OffChainActionLogged {
    /// The public key of the `ChainCard` (either User or Admin) that performed the off-chain action.
    pub actor: Pubkey,
    /// A `u64` identifier used to correlate multiple off-chain actions to a single on-chain session.
    pub session_id: u64,
    /// A `u16` code representing the specific type of off-chain action taken (e.g., 200 for HTTP OK).
    pub action_code: u16,
    /// The Unix timestamp of the logged action.
    pub ts: i64,
}
