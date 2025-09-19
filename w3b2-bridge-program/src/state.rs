use crate::errors::BridgeError;
use anchor_lang::prelude::*;

const DEFAULT_API_SIZE: usize = 10;

/// Represents the on-chain profile for a Service (Admin).
/// This PDA holds the service's configuration, price list, and collected fees.
#[account]
#[derive(Debug)]
pub struct AdminProfile {
    /// The public key of the Admin's ChainCard. This is the sole authority for this profile.
    pub authority: Pubkey,
    /// Public key for off-chain communication encryption.
    pub communication_pubkey: Pubkey,
    /// Price list for paid API calls, as a vector of (command_id, price).
    /// This vector can be dynamically resized via the `update_admin_prices` instruction.
    pub prices: Vec<(u64, u64)>,
    /// Internal balance where funds from paid API calls are collected.
    pub balance: u64,
}

/// Represents the on-chain profile for a User, linking them to a specific Admin.
/// This PDA holds the user's authorization key and their deposit balance for the service.
#[account]
#[derive(Debug)]
pub struct UserProfile {
    /// The public key of the User's ChainCard. This is the sole authority for this profile.
    pub authority: Pubkey,
    /// Public key for off-chain communication encryption.
    pub communication_pubkey: Pubkey,
    /// The deposit balance for this user, used to pay for this specific admin's services.
    pub deposit_balance: u64,
}

#[derive(Accounts)]
pub struct RegisterAdminProfile<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<AdminProfile>() + (DEFAULT_API_SIZE * std::mem::size_of::<(u64, u64)>()),
        seeds = [b"admin", authority.key().as_ref()],
        bump
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(new_prices: Vec<(u64, u64)>)]
pub struct UpdateAdminProfilePrices<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        realloc = 8 + std::mem::size_of::<AdminProfile>() + (new_prices.len() * std::mem::size_of::<(u64, u64)>()),
        realloc::payer = authority,
        realloc::zero = false,
        constraint = admin_profile.authority == authority.key() @ BridgeError::Unauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdatePricesArgs {
    pub new_prices: Vec<(u64, u64)>,
}

#[derive(Accounts)]
pub struct AdminProfileWithdraw<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::Unauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// CHECK: Safe, as it's only a destination for lamport transfers from the PDA.
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateAdminCommKey<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::Unauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
}


#[derive(Accounts)]
pub struct CloseAdminProfile<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        close = authority,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::Unauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
}

#[derive(Accounts)]
#[instruction(target_admin: Pubkey, communication_pubkey: Pubkey)] 
pub struct CreateUserProfile<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        // Увеличиваем space, чтобы вместить новый pubkey
        space = 8 + std::mem::size_of::<UserProfile>(), 
        seeds = [b"user", authority.key().as_ref(), target_admin.as_ref()],
        bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UserProfileDeposit<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        constraint = user_profile.authority == authority.key() @ BridgeError::Unauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(target_admin: Pubkey)]
pub struct UserProfileWithdraw<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), target_admin.as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::Unauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// CHECK: Safe, as it's only a destination for lamport transfers from the PDA.
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(target_admin: Pubkey)]
pub struct CloseUserProfile<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        close = authority,
        seeds = [b"user", authority.key().as_ref(), target_admin.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::Unauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
}

#[derive(Accounts)]
#[instruction(target_admin: Pubkey)]
pub struct UpdateUserCommKey<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), target_admin.as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::Unauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
}

#[derive(Accounts)]
pub struct DispatchCommand<'info> {
    pub authority: Signer<'info>, // User's ChainCard
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::Unauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
    #[account(
        mut,
        seeds = [b"admin", admin_profile.authority.as_ref()],
        bump,
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LogAction<'info> {
    pub authority: Signer<'info>,
    // The PDA is not needed here as the authority's signature is sufficient proof.
}
