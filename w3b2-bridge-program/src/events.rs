use anchor_lang::prelude::*;

/// Emitted when a new AdminProfile PDA is created.
#[event]
#[derive(Debug)]
pub struct AdminProfileRegistered {
    /// The public key of the admin's ChainCard, which is the authority of the PDA.
    pub authority: Pubkey,
    /// The public key used for off-chain communication.
    pub communication_pubkey: Pubkey,
    /// The timestamp of the registration.
    pub ts: i64,
}

/// Emitted when an admin updates their service prices.
#[event]
#[derive(Debug)]
pub struct AdminPricesUpdated {
    /// The authority of the admin profile being updated.
    pub authority: Pubkey,
    /// The new price list.
    pub new_prices: Vec<(u64, u64)>,
    /// The timestamp of the update.
    pub ts: i64,
}

/// Emitted when an admin withdraws collected fees from their profile's internal balance.
#[event]
#[derive(Debug)]
pub struct AdminFundsWithdrawn {
    /// The authority of the admin profile.
    pub authority: Pubkey,
    /// The amount of lamports withdrawn.
    pub amount: u64,
    /// The destination address for the withdrawn funds.
    pub destination: Pubkey,
    /// The timestamp of the withdrawal.
    pub ts: i64,
}

/// Emitted when an AdminProfile PDA is closed.
#[event]
#[derive(Debug)]
pub struct AdminProfileClosed {
    /// The authority of the closed admin profile.
    pub authority: Pubkey,
    /// The timestamp of the closure.
    pub ts: i64,
}

// --- User Lifecycle & Financial Events ---

/// Emitted when a new UserProfile PDA is created for a specific admin.
#[event]
#[derive(Debug)]
pub struct UserProfileCreated {
    /// The public key of the user's ChainCard, which is the authority of the PDA.
    pub authority: Pubkey,
    /// The public key of the AdminProfile this user profile is associated with.
    pub target_admin: Pubkey,
    pub communication_pubkey: Pubkey,
    /// The timestamp of the creation.
    pub ts: i64,
}

/// Emitted when a user deposits funds into their UserProfile.
#[event]
#[derive(Debug)]
pub struct FundsDeposited {
    /// The authority of the user profile.
    pub authority: Pubkey,
    /// The amount of lamports deposited.
    pub amount: u64,
    /// The user's new deposit balance after the transaction.
    pub new_deposit_balance: u64,
    /// The timestamp of the deposit.
    pub ts: i64,
}

/// Emitted when a user withdraws funds from their UserProfile.
#[event]
#[derive(Debug)]
pub struct FundsWithdrawn {
    /// The authority of the user profile.
    pub authority: Pubkey,
    /// The amount of lamports withdrawn.
    pub amount: u64,
    /// The destination address for the withdrawn funds.
    pub destination: Pubkey,
    /// The user's new deposit balance after the transaction.
    pub new_deposit_balance: u64,
    /// The timestamp of the withdrawal.
    pub ts: i64,
}

/// Emitted when a UserProfile PDA is closed.
#[event]
#[derive(Debug)]
pub struct UserProfileClosed {
    /// The authority of the closed user profile.
    pub authority: Pubkey,
    /// The timestamp of the closure.
    pub ts: i64,
}

// --- Operational Events ---

/// Emitted when a user calls a command, potentially a paid one.
#[event]
#[derive(Debug)]
pub struct CommandDispatched {
    /// The sender of the command (User's ChainCard).
    pub sender: Pubkey,
    /// The target of the command (Admin's authority Pubkey).
    pub target_admin_authority: Pubkey,
    /// The ID of the command being executed.
    pub command_id: u64,
    /// The price paid for the command in lamports (0 if it was free).
    pub price_paid: u64,
    /// Optional payload associated with the command.
    pub payload: Vec<u8>,
    /// The timestamp of the dispatch.
    pub ts: i64,
}

/// A generic event for logging off-chain actions, such as HTTP requests.
#[event]
#[derive(Debug)]
pub struct HttpActionLogged {
    /// The actor performing the action (User or Admin ChainCard).
    pub actor: Pubkey,
    /// A session identifier for correlating events.
    pub session_id: u64,
    /// A code representing the specific action taken.
    pub action_code: u16,
    /// The timestamp of the action.
    pub ts: i64,
}

#[event]
pub struct AdminCommKeyUpdated {
    pub authority: Pubkey,
    pub new_comm_pubkey: Pubkey,
    pub ts: i64,
}

#[event]
pub struct UserCommKeyUpdated {
    pub authority: Pubkey,
    pub new_comm_pubkey: Pubkey,
    pub ts: i64,
}
