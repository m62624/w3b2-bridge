//! Anchor program for W3B2 bridge.
#![allow(deprecated)]
#![allow(unexpected_cfgs)]

pub mod errors;
pub mod events;
pub mod instructions;
pub mod sm_accounts;
pub mod types;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock;

use errors::*;
use events::*;
use sm_accounts::*;
use types::*;

declare_id!("3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr");

#[program]
pub mod w3b2_bridge_program {
    use super::*;

    /// Registers an admin account.
    pub fn register_admin(ctx: Context<RegisterAdmin>, funding_amount: u64) -> Result<()> {
        instructions::register_admin(ctx, funding_amount)
    }

    /// Registers a user account.
    pub fn register_user(ctx: Context<RegisterUser>, initial_balance: u64) -> Result<()> {
        instructions::register_user(ctx, initial_balance)
    }

    /// Deactivates an admin account.
    pub fn deactivate_admin(ctx: Context<DeactivateAdmin>) -> Result<()> {
        instructions::deactivate_admin(ctx)
    }

    /// Deactivates a user account.
    pub fn deactivate_user(ctx: Context<DeactivateUser>) -> Result<()> {
        instructions::deactivate_user(ctx)
    }

    /// User requests funding.
    /// This instruction is called by the user's wallet.
    pub fn request_funding(
        ctx: Context<RequestFunding>,
        amount: u64,
        target_admin: Pubkey,
    ) -> Result<()> {
        instructions::request_funding(ctx, amount, target_admin)
    }

    /// Admin approves and funds a user's request.
    /// This is called by the service's admin wallet.
    pub fn approve_funding(ctx: Context<ApproveFunding>) -> Result<()> {
        instructions::approve_funding(ctx)
    }

    pub fn dispatch_command(
        ctx: Context<DispatchCommand>,
        command_id: u64,
        mode: CommandMode,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::dispatch_command(ctx, command_id, mode, payload)
    }
}
