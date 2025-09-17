//! Anchor program for W3B2 bridge.
#![allow(deprecated)]
#![allow(unexpected_cfgs)]

pub mod errors;
pub mod events;
pub mod instructions;
pub mod protocol;
pub mod state;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock;

use errors::*;
use events::*;
use protocol::*;
use state::*;

declare_id!("3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr");

#[program]
pub mod w3b2_bridge_program {
    use super::*;

    /// Registers an admin account with an initial balance and a communication public key.
    pub fn register_admin(
        ctx: Context<RegisterAdmin>,
        initial_balance: u64,
        communication_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::register_admin(ctx, initial_balance, communication_pubkey)
    }

    /// Registers a user account with an initial balance and a communication public key.
    pub fn register_user(
        ctx: Context<RegisterUser>,
        initial_balance: u64,
        communication_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::register_user(ctx, initial_balance, communication_pubkey)
    }

    /// Deactivates an admin account. Requires multi-sig.
    pub fn deactivate_admin(ctx: Context<DeactivateAdmin>) -> Result<()> {
        instructions::deactivate_admin(ctx)
    }

    /// Deactivates a user account. Requires multi-sig.
    pub fn deactivate_user(ctx: Context<DeactivateUser>) -> Result<()> {
        instructions::deactivate_user(ctx)
    }

    /// User requests funding from a target admin.
    pub fn request_funding(
        ctx: Context<RequestFunding>,
        amount: u64,
        target_admin: Pubkey,
    ) -> Result<()> {
        instructions::request_funding(ctx, amount, target_admin)
    }

    /// Admin approves and funds a user's request. Requires multi-sig from the admin.
    pub fn approve_funding(ctx: Context<ApproveFunding>) -> Result<()> {
        instructions::approve_funding(ctx)
    }

    /// Dispatches a command from a sender (Admin/User) to a recipient (Admin/User).
    pub fn dispatch_command(
        ctx: Context<DispatchCommand>,
        command_id: u64,
        mode: CommandMode,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::dispatch_command(ctx, command_id, mode, payload)
    }

    /// Logs a simple off-chain action to the blockchain. Requires multi-sig.
    pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
        instructions::log_action(ctx, session_id, action_code)
    }

    /// Updates the communication public key for an admin account. Requires multi-sig.
    pub fn update_admin_comm_key(ctx: Context<UpdateAdminCommKey>, new_key: Pubkey) -> Result<()> {
        instructions::update_admin_comm_key(ctx, new_key)
    }

    /// Updates the communication public key for a user account. Requires multi-sig.
    pub fn update_user_comm_key(ctx: Context<UpdateUserCommKey>, new_key: Pubkey) -> Result<()> {
        instructions::update_user_comm_key(ctx, new_key)
    }
}
