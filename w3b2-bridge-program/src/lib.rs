//! The on-chain program for the W3B2 Bridge protocol.
//!
//! This crate defines the core instruction interface for creating and managing
//! service provider (`Admin`) and client (`User`) profiles, handling financial
//! interactions like deposits and withdrawals, and facilitating a secure,
//! bidirectional command dispatch system between off-chain parties.

#![allow(deprecated)]
#![allow(unexpected_cfgs)]

pub mod errors;
pub mod events;
pub mod instructions;
pub mod protocols;
pub mod state;

use anchor_lang::prelude::*;
use errors::*;
use events::*;
use state::*;

declare_id!("w3b2d133a2a61a7a1a2b1b3c1d1e1f1a1b2c3d4e5f6");

/// Defines the primary instruction interface for the W3B2 Bridge program.
/// Each public function in this module corresponds to a callable on-chain instruction.
#[program]
pub mod w3b2_bridge_program {
    use super::*;

    // --- Admin Instructions ---

    /// Initializes a new `AdminProfile` PDA for a service provider. This instruction
    /// creates the on-chain representation of a service, setting its owner (`authority`)
    /// and initial configuration.
    ///
    /// # Arguments
    /// * `ctx` - The context of accounts for registering an admin.
    /// * `communication_pubkey` - The public key the admin will use for off-chain communication.
    pub fn admin_register_profile(
        ctx: Context<AdminRegisterProfile>,
        communication_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::admin_register_profile(ctx, communication_pubkey)
    }

    /// Updates the `communication_pubkey` for an existing `AdminProfile`.
    /// This allows a service provider to rotate their off-chain encryption keys.
    ///
    /// # Arguments
    /// * `ctx` - The context of accounts for updating the key.
    /// * `new_key` - The new `Pubkey` to set as the communication key.
    pub fn admin_update_comm_key(ctx: Context<AdminUpdateCommKey>, new_key: Pubkey) -> Result<()> {
        instructions::admin_update_comm_key(ctx, new_key)
    }

    /// Closes an `AdminProfile` account and refunds its rent lamports to the owner.
    /// This effectively unregisters a service from the protocol.
    ///
    /// # Arguments
    /// * `ctx` - The context containing the `authority` and the `admin_profile` to be closed.
    pub fn admin_close_profile(ctx: Context<AdminCloseProfile>) -> Result<()> {
        instructions::admin_close_profile(ctx)
    }

    /// Updates the price list for an admin's services. The associated `AdminProfile`
    /// account is automatically resized to fit the new list.
    ///
    /// # Arguments
    /// * `ctx` - The context of accounts for updating the price list.
    /// * `args` - A struct containing `new_prices`, a `Vec<(u64, u64)>` of (command_id, price).
    pub fn admin_update_prices(
        ctx: Context<AdminUpdatePrices>,
        args: UpdatePricesArgs,
    ) -> Result<()> {
        instructions::admin_update_prices(ctx, args.new_prices)
    }

    /// Allows an admin to withdraw earned funds from their `AdminProfile`'s internal balance
    /// to a specified destination wallet.
    ///
    /// # Arguments
    /// * `ctx` - The context of accounts for the withdrawal.
    /// * `amount` - The number of lamports to withdraw.
    pub fn admin_withdraw(ctx: Context<AdminWithdraw>, amount: u64) -> Result<()> {
        instructions::admin_withdraw(ctx, amount)
    }

    /// Allows an admin to send a command or notification to a user. This is a non-financial
    /// transaction; its primary purpose is to emit an `AdminCommandDispatched` event that
    /// an off-chain user `connector` can listen and react to.
    ///
    /// # Arguments
    /// * `ctx` - The context, including the admin's `authority`, their `admin_profile`, and the target `user_profile`.
    /// * `command_id` - The `u64` identifier of the admin's command.
    /// * `payload` - An opaque `Vec<u8>` for application-specific data.
    pub fn admin_dispatch_command(
        ctx: Context<AdminDispatchCommand>,
        command_id: u64,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::admin_dispatch_command(ctx, command_id, payload)
    }

    // --- User Instructions ---

    /// Creates a `UserProfile` PDA, linking a user's `ChainCard` to a specific admin service.
    ///
    /// # Arguments
    /// * `ctx` - The context of accounts for creating a user profile.
    /// * `target_admin` - The `Pubkey` of the `AdminProfile` PDA this user is registering with.
    /// * `communication_pubkey` - The user's public key for off-chain communication.
    pub fn user_create_profile(
        ctx: Context<UserCreateProfile>,
        target_admin: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::user_create_profile(ctx, target_admin, communication_pubkey)
    }

    /// Updates the `communication_pubkey` for an existing `UserProfile`.
    ///
    /// # Arguments
    /// * `ctx` - The context of accounts for updating the key.
    /// * `new_key` - The new `Pubkey` to set as the communication key.
    pub fn user_update_comm_key(ctx: Context<UserUpdateCommKey>, new_key: Pubkey) -> Result<()> {
        instructions::user_update_comm_key(ctx, new_key)
    }

    /// Closes a `UserProfile` account. All remaining lamports (both from the deposit
    /// balance and for rent) are automatically returned to the user's `authority`.
    ///
    /// # Arguments
    /// * `ctx` - The context containing the user's `authority` and the `user_profile` to be closed.
    pub fn user_close_profile(ctx: Context<UserCloseProfile>) -> Result<()> {
        instructions::user_close_profile(ctx)
    }

    /// Allows a user to deposit lamports into their `UserProfile` PDA to pre-fund
    /// future payments for a service.
    ///
    /// # Arguments
    /// * `ctx` - The context of accounts for the deposit.
    /// * `amount` - The number of lamports to deposit.
    pub fn user_deposit(ctx: Context<UserDeposit>, amount: u64) -> Result<()> {
        instructions::user_deposit(ctx, amount)
    }

    /// Allows a user to withdraw unspent funds from their `UserProfile`'s deposit balance.
    ///
    /// # Arguments
    /// * `ctx` - The context of accounts for the withdrawal.
    /// * `amount` - The number of lamports to withdraw.
    pub fn user_withdraw(ctx: Context<UserWithdraw>, amount: u64) -> Result<()> {
        instructions::user_withdraw(ctx, amount)
    }

    // --- Operational Instructions ---

    /// The primary instruction for a user to call a service's API. If the command is priced,
    /// it handles payment by debiting the user's deposit and crediting the admin's balance.
    ///
    /// # Arguments
    /// * `ctx` - The context, including the user's `authority`, their `user_profile`, and the target `admin_profile`.
    /// * `command_id` - The `u64` identifier of the service's command to be executed.
    /// * `payload` - An opaque `Vec<u8>` containing serialized, application-specific data for the off-chain service.
    pub fn user_dispatch_command(
        ctx: Context<UserDispatchCommand>,
        command_id: u64,
        payload: Vec<u8>,
    ) -> Result<()> {
        instructions::user_dispatch_command(ctx, command_id, payload)
    }

    /// A generic instruction to log a significant off-chain action to the blockchain,
    /// creating an immutable, auditable record.
    ///
    /// # Arguments
    /// * `ctx` - The context, containing the `Signer` who is the actor.
    /// * `session_id` - A `u64` identifier to correlate this action with a session.
    /// * `action_code` - A `u16` code representing the specific off-chain action.
    pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
        instructions::log_action(ctx, session_id, action_code)
    }
}
