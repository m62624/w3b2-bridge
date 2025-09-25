use super::*;
use crate::instructions::solana_program::program::invoke;
use crate::instructions::solana_program::system_instruction;
use anchor_lang::solana_program;
// use solana_program::{program::invoke, system_instruction};

/// The maximum size in bytes for the `payload` in dispatch instructions.
pub const MAX_PAYLOAD_SIZE: usize = 1000;

// --- Admin Instructions ---

/// Initializes a new `AdminProfile` PDA for a service provider.
/// This function sets the initial state of the admin's on-chain profile.
pub fn admin_register_profile(
    ctx: Context<AdminRegisterProfile>,
    communication_pubkey: Pubkey,
) -> Result<()> {
    let admin_profile = &mut ctx.accounts.admin_profile;
    admin_profile.authority = ctx.accounts.authority.key();
    admin_profile.communication_pubkey = communication_pubkey;
    admin_profile.prices = Vec::new();
    admin_profile.balance = 0;

    emit!(AdminProfileRegistered {
        authority: admin_profile.authority,
        communication_pubkey: admin_profile.communication_pubkey,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Updates the off-chain communication public key for an `AdminProfile`.
pub fn admin_update_comm_key(ctx: Context<AdminUpdateCommKey>, new_key: Pubkey) -> Result<()> {
    ctx.accounts.admin_profile.communication_pubkey = new_key;
    emit!(AdminCommKeyUpdated {
        authority: ctx.accounts.authority.key(),
        new_comm_pubkey: new_key,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Closes an `AdminProfile` account.
/// The `close` directive in the `AdminCloseProfile` struct ensures all lamports
/// are safely returned to the admin's authority (`ChainCard`).
pub fn admin_close_profile(ctx: Context<AdminCloseProfile>) -> Result<()> {
    emit!(AdminProfileClosed {
        authority: ctx.accounts.authority.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Updates the price list for an admin's services.
/// The associated `AdminProfile` account is automatically resized by Anchor
/// to accommodate the new list size.
pub fn admin_update_prices(
    ctx: Context<AdminUpdatePrices>,
    mut new_prices: Vec<(u16, u64)>,
) -> Result<()> {
    new_prices.sort_unstable_by_key(|k| k.0);
    new_prices.dedup_by_key(|k| k.0);
    ctx.accounts.admin_profile.prices = new_prices.clone();
    emit!(AdminPricesUpdated {
        authority: ctx.accounts.authority.key(),
        new_prices,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Allows an admin to withdraw earned funds from their `AdminProfile`'s internal balance.
/// It performs checks to ensure the withdrawal does not violate the rent-exemption rule.
pub fn admin_withdraw(ctx: Context<AdminWithdraw>, amount: u64) -> Result<()> {
    let admin_profile = &mut ctx.accounts.admin_profile;
    let destination = &ctx.accounts.destination;

    // Check if the internal balance is sufficient.
    require!(
        admin_profile.balance >= amount,
        BridgeError::InsufficientAdminBalance
    );

    // Check if the on-chain lamport balance will remain above the rent-exempt minimum.
    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(admin_profile.to_account_info().data_len());
    require!(
        admin_profile.to_account_info().lamports() - amount >= rent_exempt_minimum,
        BridgeError::RentExemptViolation
    );

    // Perform the lamport transfer by directly debiting and crediting the accounts.
    **admin_profile.to_account_info().try_borrow_mut_lamports()? -= amount;
    **destination.to_account_info().try_borrow_mut_lamports()? += amount;

    // Update the internal balance state.
    admin_profile.balance -= amount;

    emit!(AdminFundsWithdrawn {
        authority: admin_profile.authority,
        amount,
        destination: destination.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Allows an admin to send a command or notification to a user.
/// This is a non-financial transaction; its primary purpose is to emit an event
/// that an off-chain user `connector` can listen and react to.
pub fn admin_dispatch_command(
    ctx: Context<AdminDispatchCommand>,
    command_id: u64,
    payload: Vec<u8>,
) -> Result<()> {
    require!(
        payload.len() <= MAX_PAYLOAD_SIZE,
        BridgeError::PayloadTooLarge
    );

    emit!(AdminCommandDispatched {
        sender: ctx.accounts.admin_authority.key(),
        target_user_authority: ctx.accounts.user_profile.authority,
        command_id,
        payload,
        ts: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

// --- User Instructions ---

/// Creates a `UserProfile` PDA, linking a user's `ChainCard` to a specific admin service.
/// The `admin_authority_on_creation` field is set to the admin's PDA key to create a
/// permanent, verifiable link.
pub fn user_create_profile(
    ctx: Context<UserCreateProfile>,
    target_admin: Pubkey,
    communication_pubkey: Pubkey,
) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;
    user_profile.authority = ctx.accounts.authority.key();
    user_profile.deposit_balance = 0;
    user_profile.communication_pubkey = communication_pubkey;
    user_profile.admin_authority_on_creation = target_admin;

    emit!(UserProfileCreated {
        authority: user_profile.authority,
        target_admin,
        communication_pubkey,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Updates the off-chain communication public key for a `UserProfile`.
pub fn user_update_comm_key(ctx: Context<UserUpdateCommKey>, new_key: Pubkey) -> Result<()> {
    ctx.accounts.user_profile.communication_pubkey = new_key;
    emit!(UserCommKeyUpdated {
        authority: ctx.accounts.authority.key(),
        new_comm_pubkey: new_key,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Closes a `UserProfile` account.
/// All remaining lamports (both from the deposit balance and for rent) are
/// automatically returned to the user's `authority` (`ChainCard`).
pub fn user_close_profile(_ctx: Context<UserCloseProfile>) -> Result<()> {
    emit!(UserProfileClosed {
        authority: _ctx.accounts.authority.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Allows a user to deposit lamports into their `UserProfile` PDA.
/// This pre-funds their account to pay for future service calls.
pub fn user_deposit(ctx: Context<UserDeposit>, amount: u64) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;

    // Perform a Cross-Program Invocation (CPI) to the System Program to transfer lamports
    // from the user's `authority` wallet to the `user_profile` PDA.
    invoke(
        &system_instruction::transfer(
            &ctx.accounts.authority.key(),
            &user_profile.to_account_info().key(),
            amount,
        ),
        &[
            ctx.accounts.authority.to_account_info(),
            user_profile.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // Update the internal deposit balance state.
    user_profile.deposit_balance += amount;

    emit!(UserFundsDeposited {
        authority: user_profile.authority,
        amount,
        new_deposit_balance: user_profile.deposit_balance,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Allows a user to withdraw unspent funds from their `UserProfile` deposit balance.
pub fn user_withdraw(ctx: Context<UserWithdraw>, amount: u64) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;
    let destination = &ctx.accounts.destination;

    // Check if the internal deposit balance is sufficient.
    require!(
        user_profile.deposit_balance >= amount,
        BridgeError::InsufficientDepositBalance
    );

    // Check if the on-chain lamport balance will remain above the rent-exempt minimum.
    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(user_profile.to_account_info().data_len());
    require!(
        user_profile.to_account_info().lamports() - amount >= rent_exempt_minimum,
        BridgeError::RentExemptViolation
    );

    // Perform the lamport transfer.
    **user_profile.to_account_info().try_borrow_mut_lamports()? -= amount;
    **destination.to_account_info().try_borrow_mut_lamports()? += amount;

    // Update the internal deposit balance state.
    user_profile.deposit_balance -= amount;

    emit!(UserFundsWithdrawn {
        authority: user_profile.authority,
        amount,
        destination: destination.key(),
        new_deposit_balance: user_profile.deposit_balance,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

// --- Operational Instructions ---

/// The primary instruction for a user to call a service's API.
/// If the called command has a price, this instruction handles the payment by
/// transferring lamports from the `UserProfile` PDA to the `AdminProfile` PDA.
pub fn user_dispatch_command(
    ctx: Context<UserDispatchCommand>,
    command_id: u16,
    payload: Vec<u8>,
) -> Result<()> {
    require!(
        payload.len() <= MAX_PAYLOAD_SIZE,
        BridgeError::PayloadTooLarge
    );

    let user_profile = &mut ctx.accounts.user_profile;
    let admin_profile = &mut ctx.accounts.admin_profile;

    let command_price = match admin_profile
        .prices
        .binary_search_by_key(&command_id, |&(id, _)| id)
    {
        Ok(index) => admin_profile.prices[index].1,
        Err(_) => 0,
    };

    // If the command is not free, process the payment.
    if command_price > 0 {
        require!(
            user_profile.deposit_balance >= command_price,
            BridgeError::InsufficientDepositBalance
        );

        let rent = Rent::get()?;
        let rent_exempt_minimum = rent.minimum_balance(user_profile.to_account_info().data_len());
        require!(
            user_profile.to_account_info().lamports() - command_price >= rent_exempt_minimum,
            BridgeError::RentExemptViolation
        );

        // Transfer lamports from the user's PDA to the admin's PDA.
        **user_profile.to_account_info().try_borrow_mut_lamports()? -= command_price;
        **admin_profile.to_account_info().try_borrow_mut_lamports()? += command_price;

        // Update the internal balances of both profiles.
        user_profile.deposit_balance -= command_price;
        admin_profile.balance += command_price;
    }

    emit!(UserCommandDispatched {
        sender: ctx.accounts.authority.key(),
        target_admin_authority: admin_profile.authority,
        command_id,
        price_paid: command_price,
        payload,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// A generic instruction to log a significant off-chain action to the blockchain.
/// This creates an immutable, auditable record of events that happen outside the chain.
pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
    emit!(OffChainActionLogged {
        actor: ctx.accounts.authority.key(),
        session_id,
        action_code,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
