// src/instructions.rs

use super::*;
use solana_program::{program::invoke, program::invoke_signed, system_instruction};

const MAX_PAYLOAD_SIZE: usize = 1024;

// --- Admin Profile Instructions ---

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

pub fn admin_update_comm_key(ctx: Context<AdminUpdateCommKey>, new_key: Pubkey) -> Result<()> {
    ctx.accounts.admin_profile.communication_pubkey = new_key;
    emit!(AdminCommKeyUpdated {
        authority: ctx.accounts.authority.key(),
        new_comm_pubkey: new_key,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

pub fn admin_close_profile(ctx: Context<AdminCloseProfile>) -> Result<()> {
    emit!(AdminProfileClosed {
        authority: ctx.accounts.authority.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

pub fn admin_update_prices(
    ctx: Context<AdminUpdatePrices>,
    new_prices: Vec<(u64, u64)>,
) -> Result<()> {
    ctx.accounts.admin_profile.prices = new_prices.clone();
    emit!(AdminPricesUpdated {
        authority: ctx.accounts.authority.key(),
        new_prices,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

pub fn admin_withdraw(ctx: Context<AdminWithdraw>, amount: u64) -> Result<()> {
    let admin_profile = &mut ctx.accounts.admin_profile;
    let authority = &ctx.accounts.authority;
    let destination = &ctx.accounts.destination;

    require!(
        admin_profile.balance >= amount,
        BridgeError::InsufficientPDABalance
    );

    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(admin_profile.to_account_info().data_len());
    require!(
        admin_profile.to_account_info().lamports() - amount >= rent_exempt_minimum,
        BridgeError::RentExemptViolation
    );

    let bump = ctx.bumps.admin_profile;
    let authority_key = authority.key();
    let seeds = &[&b"admin"[..], authority_key.as_ref(), &[bump]];

    invoke_signed(
        &system_instruction::transfer(
            &admin_profile.to_account_info().key(),
            &destination.key(),
            amount,
        ),
        &[
            admin_profile.to_account_info(),
            destination.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[&seeds[..]],
    )?;

    admin_profile.balance -= amount;

    emit!(AdminFundsWithdrawn {
        authority: admin_profile.authority,
        amount,
        destination: destination.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Creates a UserProfile PDA, linking a user's ChainCard to a specific admin service.
pub fn user_create_profile(
    ctx: Context<UserCreateProfile>,
    target_admin: Pubkey,
    communication_pubkey: Pubkey,
) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;
    user_profile.authority = ctx.accounts.authority.key();
    user_profile.deposit_balance = 0;
    user_profile.communication_pubkey = communication_pubkey;

    emit!(UserProfileCreated {
        authority: user_profile.authority,
        target_admin,
        communication_pubkey,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

pub fn user_update_comm_key(
    ctx: Context<UserUpdateCommKey>,
    _target_admin: Pubkey,
    new_key: Pubkey,
) -> Result<()> {
    ctx.accounts.user_profile.communication_pubkey = new_key;
    emit!(UserCommKeyUpdated {
        authority: ctx.accounts.authority.key(),
        new_comm_pubkey: new_key,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

pub fn user_close_profile(_ctx: Context<UserCloseProfile>, _target_admin: Pubkey) -> Result<()> {
    emit!(UserProfileClosed {
        authority: _ctx.accounts.authority.key(),
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

pub fn user_deposit(ctx: Context<UserDeposit>, amount: u64) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;

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

    user_profile.deposit_balance += amount;

    emit!(FundsDeposited {
        authority: user_profile.authority,
        amount,
        new_deposit_balance: user_profile.deposit_balance,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

pub fn user_withdraw(ctx: Context<UserWithdraw>, amount: u64, target_admin: Pubkey) -> Result<()> {
    let user_profile = &mut ctx.accounts.user_profile;
    let authority = &ctx.accounts.authority;
    let destination = &ctx.accounts.destination;

    require!(
        user_profile.deposit_balance >= amount,
        BridgeError::InsufficientDepositBalance
    );

    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(user_profile.to_account_info().data_len());
    require!(
        user_profile.to_account_info().lamports() - amount >= rent_exempt_minimum,
        BridgeError::RentExemptViolation
    );

    let bump = ctx.bumps.user_profile;
    let authority_key = authority.key();
    let seeds = &[
        &b"user"[..],
        authority_key.as_ref(),
        target_admin.as_ref(),
        &[bump],
    ];

    invoke_signed(
        &system_instruction::transfer(
            &user_profile.to_account_info().key(),
            &destination.key(),
            amount,
        ),
        &[
            user_profile.to_account_info(),
            destination.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[&seeds[..]],
    )?;

    user_profile.deposit_balance -= amount;

    emit!(FundsWithdrawn {
        authority: user_profile.authority,
        amount,
        destination: destination.key(),
        new_deposit_balance: user_profile.deposit_balance,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

// --- Operational Instructions ---

pub fn dispatch_command(
    ctx: Context<DispatchCommand>,
    command_id: u64,
    payload: Vec<u8>,
) -> Result<()> {
    require!(
        payload.len() <= MAX_PAYLOAD_SIZE,
        BridgeError::PayloadTooLarge
    );

    let user_profile = &mut ctx.accounts.user_profile;
    let admin_profile = &mut ctx.accounts.admin_profile;

    let command_price = admin_profile
        .prices
        .iter()
        .find(|&&(id, _)| id == command_id)
        .map(|&(_, price)| price)
        .unwrap_or(0);

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

        let user_bump = ctx.bumps.user_profile;
        let authority_key = ctx.accounts.authority.key();
        let admin_profile_key = admin_profile.to_account_info().key();
        let user_seeds = &[
            &b"user"[..],
            authority_key.as_ref(),
            admin_profile_key.as_ref(),
            &[user_bump],
        ];

        invoke_signed(
            &system_instruction::transfer(
                &user_profile.to_account_info().key(),
                &admin_profile.to_account_info().key(),
                command_price,
            ),
            &[
                user_profile.to_account_info(),
                admin_profile.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[&user_seeds[..]],
        )?;

        user_profile.deposit_balance -= command_price;
        admin_profile.balance += command_price;
    }

    emit!(CommandDispatched {
        sender: ctx.accounts.authority.key(),
        target_admin_authority: admin_profile.authority,
        command_id,
        price_paid: command_price,
        payload,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
    emit!(HttpActionLogged {
        actor: ctx.accounts.authority.key(),
        session_id,
        action_code,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
