//! Anchor program for W3B2 bridge.
#![allow(deprecated)]
#![allow(unexpected_cfgs)]

pub mod errors;
pub mod events;
pub mod instructions;
pub mod protocol;
pub mod state;

use anchor_lang::prelude::*;

use errors::*;
use events::*;
use state::*;

// Program's on-chain address.
declare_id!("w3b2d133a2a61a7a1a2b1b3c1d1e1f1a1b2c3d4e5f6"); // Replace with your new program ID

#[program]
pub mod w3b2_bridge_program {
    use super::*;

    // --- Admin Instructions ---

    /// Creates an AdminProfile PDA to represent a service on the blockchain.
    /// This profile holds the service's authority, communication key, and price list.
    pub fn admin_register_profile(
        ctx: Context<AdminRegisterProfile>,
        communication_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::admin_register_profile(ctx, communication_pubkey)
    }

    /// Updates the off-chain communication public key for an admin.
    pub fn admin_update_comm_key(ctx: Context<AdminUpdateCommKey>, new_key: Pubkey) -> Result<()> {
        instructions::admin_update_comm_key(ctx, new_key)
    }

    /// Closes the AdminProfile and returns all lamports to the authority.
    pub fn admin_close_profile(ctx: Context<AdminCloseProfile>) -> Result<()> {
        instructions::admin_close_profile(ctx)
    }

    /// Updates the price list for an admin's services.
    /// The account is automatically resized to fit the new list.
    pub fn admin_update_prices(
        ctx: Context<AdminUpdatePrices>,
        args: UpdatePricesArgs,
    ) -> Result<()> {
        instructions::admin_update_prices(ctx, args.new_prices)
    }

    /// Withdraws collected fees from the AdminProfile to a destination wallet.
    pub fn admin_withdraw(ctx: Context<AdminWithdraw>, amount: u64) -> Result<()> {
        instructions::admin_withdraw(ctx, amount)
    }

    // --- User Instructions ---

    /// Creates a UserProfile PDA, linking a user's ChainCard to a specific admin service.
    pub fn user_create_profile(
        ctx: Context<UserCreateProfile>,
        target_admin: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::user_create_profile(ctx, target_admin, communication_pubkey)
    }

    /// Updates the off-chain communication public key for a user.
    pub fn user_update_comm_key(
        ctx: Context<UserUpdateCommKey>,
        target_admin: Pubkey,
        new_key: Pubkey,
    ) -> Result<()> {
        instructions::user_update_comm_key(ctx, target_admin, new_key)
    }

    /// Deposits lamports from a user's ChainCard into their UserProfile PDA.
    pub fn user_deposit(ctx: Context<UserDeposit>, amount: u64) -> Result<()> {
        instructions::user_deposit(ctx, amount)
    }
    /// Withdraws lamports from a user's UserProfile PDA to a destination wallet.
    pub fn user_withdraw(
        ctx: Context<UserWithdraw>,
        amount: u64,
        target_admin: Pubkey,
    ) -> Result<()> {
        instructions::user_withdraw(ctx, amount, target_admin)
    }

    // --- Operational Instructions ---

    /// The main instruction for a user to call a service's API.
    /// Handles payment by debiting the user's deposit and crediting the admin's balance.
    pub fn dispatch_command(
        ctx: Context<DispatchCommand>,
        command_id: u64,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::dispatch_command(ctx, command_id, payload)
    }

    /// Logs a significant off-chain action to the blockchain for auditing purposes.
    pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
        instructions::log_action(ctx, session_id, action_code)
    }
}
