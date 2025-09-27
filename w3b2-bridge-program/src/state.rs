use crate::errors::BridgeError;
use anchor_lang::prelude::*;

/// The default number of price entries to allocate space for when creating an AdminProfile.
const DEFAULT_API_SIZE: usize = 10;

// --- Account Data Structs ---

/// Represents the on-chain profile for a Service Provider (Admin).
/// This PDA holds the service's configuration, price list, and collected fees.
#[account]
#[derive(Debug)]
pub struct AdminProfile {
    /// The public key of the Admin's `ChainCard`, which is the sole authority
    /// allowed to manage this profile.
    pub authority: Pubkey,
    /// A public key provided by the admin for secure off-chain key exchange,
    /// typically used for hybrid encryption with clients.
    pub communication_pubkey: Pubkey,
    /// A dynamic list of `(command_id, price)` tuples that defines the cost
    /// in lamports for various off-chain services.
    pub prices: Vec<PriceEntry>,
    /// The internal balance in lamports where fees from paid user commands are collected.
    /// This balance can be withdrawn by the admin.
    pub balance: u64,
}

/// Represents a user's on-chain relationship with and deposit for a specific Admin service.
/// This PDA holds the user's authorization key and their prepaid balance.
#[account]
#[derive(Debug)]
pub struct UserProfile {
    /// The public key of the User's `ChainCard`, which is the sole authority
    /// allowed to manage this profile.
    pub authority: Pubkey,
    /// A public key provided by the user for secure off-chain key exchange.
    pub communication_pubkey: Pubkey,
    /// The public key of the `AdminProfile` PDA this user profile was created for.
    /// This field permanently links the user's profile to a specific service.
    pub admin_authority_on_creation: Pubkey,
    /// The user's prepaid balance in lamports for this specific service. This balance
    /// is debited by the `user_dispatch_command` instruction.
    pub deposit_balance: u64,
}

// --- Instruction Accounts Structs ---

// --- Admin Instructions ---

/// Defines the accounts required for the `admin_register_profile` instruction.
#[derive(Accounts)]
pub struct AdminRegisterProfile<'info> {
    /// The `Signer` who will become the owner of the new `AdminProfile`. This is the admin's `ChainCard`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The new `AdminProfile` account to be initialized. Its address is a PDA
    /// derived from the `authority`'s key.
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<AdminProfile>() + (DEFAULT_API_SIZE * std::mem::size_of::<(u64, u64)>()),
        seeds = [b"admin", authority.key().as_ref()],
        bump
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The Solana System Program, required by Anchor for account creation (`init`).
    pub system_program: Program<'info, System>,
}

/// Defines the accounts for the `admin_update_prices` instruction.
#[derive(Accounts)]
#[instruction(args: UpdatePricesArgs)]
pub struct AdminUpdatePrices<'info> {
    /// The admin's `ChainCard`, who must be the `authority` of the `admin_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` account to be updated. Constraints verify the `authority`
    /// and the account's PDA seeds. The account will be resized (`realloc`) to
    /// fit the new price list.
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        realloc = 8 + std::mem::size_of::<AdminProfile>() + (args.new_prices.len() * std::mem::size_of::<(u64, u64)>()),
        realloc::payer = authority,
        realloc::zero = false,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The System Program, required by Anchor for `realloc`.
    pub system_program: Program<'info, System>,
}

/// Represents a single entry in an admin's price list.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub struct PriceEntry {
    /// Identifier of the command (stable u16).
    pub command_id: u16,
    /// Price in lamports.
    pub price: u64,
}

impl PriceEntry {
    pub fn new(command_id: u16, price: u64) -> Self {
        Self { command_id, price }
    }
}

/// A container struct for instruction arguments that involve a `Vec`.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdatePricesArgs {
    /// The new price list to set for the admin's services.
    pub new_prices: Vec<PriceEntry>,
}

/// Defines the accounts for the `admin_withdraw` instruction.
#[derive(Accounts)]
pub struct AdminWithdraw<'info> {
    /// The admin's `ChainCard`, who must be the `authority` of the `admin_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` from which funds will be withdrawn. Constraints
    /// verify the `authority` and the PDA seeds.
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The account that will receive the withdrawn lamports. It is marked as `mut`
    /// because its lamport balance will be increased.
    /// CHECK: This is safe because it's only used as a destination for a lamport transfer
    /// from a program-controlled PDA, and does not require data deserialization.
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    /// The System Program, required for the underlying lamport transfer.
    pub system_program: Program<'info, System>,
}

/// Defines the accounts for the `admin_update_comm_key` instruction.
#[derive(Accounts)]
pub struct AdminUpdateCommKey<'info> {
    /// The admin's `ChainCard`, who must be the `authority` of the `admin_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` account to be updated. Constraints verify the `authority`
    /// and the account's PDA seeds.
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
}

/// Defines the accounts for the `admin_close_profile` instruction.
#[derive(Accounts)]
pub struct AdminCloseProfile<'info> {
    /// The admin's `ChainCard`, who must be the `authority` of the `admin_profile`.
    /// This account will receive the rent lamports back from the closed account.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` account to be closed. Constraints verify the `authority`
    /// and PDA seeds. The `close` directive tells Anchor to return all lamports
    /// from this account to the `authority`.
    #[account(
        mut,
        close = authority,
        seeds = [b"admin", authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
}

/// Defines the accounts for the `admin_dispatch_command` instruction.
#[derive(Accounts)]
pub struct AdminDispatchCommand<'info> {
    /// The `Signer` of the transaction. This must be the `ChainCard` of the admin.
    pub admin_authority: Signer<'info>,
    /// The admin's own profile PDA. Constraints ensure that the `admin_authority`
    /// is the legitimate owner of this profile.
    #[account(
        seeds = [b"admin", admin_authority.key().as_ref()],
        bump,
        constraint = admin_profile.authority == admin_authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The target `UserProfile` to which the command is being sent. A constraint
    /// ensures this profile is associated with this specific `admin_profile`.
    #[account(
        constraint = user_profile.admin_authority_on_creation == admin_profile.key() @ BridgeError::AdminMismatch
    )]
    pub user_profile: Account<'info, UserProfile>,
}

// --- User Instructions ---

/// Defines the accounts for the `user_create_profile` instruction.
#[derive(Accounts)]
#[instruction(target_admin: Pubkey, communication_pubkey: Pubkey)]
pub struct UserCreateProfile<'info> {
    /// The `Signer` who will become the owner of the new `UserProfile`. This is the user's `ChainCard`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The new `UserProfile` account to be initialized. Its address is a PDA
    /// derived from the user's `authority` key and the `target_admin` PDA key.
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<UserProfile>(),
        seeds = [b"user", authority.key().as_ref(), target_admin.as_ref()],
        bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The Solana System Program, required by Anchor for account creation (`init`).
    pub system_program: Program<'info, System>,
}

/// Defines the accounts for the `user_deposit` instruction.
#[derive(Accounts)]
pub struct UserDeposit<'info> {
    /// The user's `ChainCard`, who must be the `authority` of the `user_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`. This is required
    /// to derive and verify the `user_profile` PDA address.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` to receive the deposit. Constraints verify the PDA seeds
    /// (linking it to the `authority` and `admin_profile`) and ownership.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The System Program, required for the underlying lamport transfer.
    pub system_program: Program<'info, System>,
}

/// Defines the accounts for the `user_withdraw` instruction.
#[derive(Accounts)]
pub struct UserWithdraw<'info> {
    /// The user's `ChainCard`, who must be the `authority` of the `user_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` from which funds will be withdrawn.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The account that will receive the withdrawn lamports.
    /// CHECK: This is safe because it's only a destination for a lamport transfer.
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    /// The System Program, required for the underlying lamport transfer.
    pub system_program: Program<'info, System>,
}

/// Defines the accounts for the `user_update_comm_key` instruction.
#[derive(Accounts)]
pub struct UserUpdateCommKey<'info> {
    /// The user's `ChainCard`, who must be the `authority` of the `user_profile`.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` account to be updated.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
}

/// Defines the accounts for the `user_close_profile` instruction.
#[derive(Accounts)]
pub struct UserCloseProfile<'info> {
    /// The user's `ChainCard`, who must be the `authority` of the `user_profile`.
    /// This account will receive the refunded lamports.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The `AdminProfile` associated with the `user_profile`.
    pub admin_profile: Account<'info, AdminProfile>,
    /// The `UserProfile` account to be closed. The `close` directive will transfer
    /// all its lamports to the `authority`.
    #[account(
        mut,
        close = authority,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
}

/// Defines the accounts for the `user_dispatch_command` instruction.
#[derive(Accounts)]
pub struct UserDispatchCommand<'info> {
    /// The `Signer` of the transaction. This is the user's `ChainCard`.
    pub authority: Signer<'info>,
    /// The user's profile PDA. Constraints ensure the `authority` is the owner
    /// and that this profile is linked to the provided `admin_profile` via its seeds.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), admin_profile.key().as_ref()],
        bump,
        constraint = user_profile.authority == authority.key() @ BridgeError::SignerUnauthorized
    )]
    pub user_profile: Account<'info, UserProfile>,
    /// The target `AdminProfile` of the service being called. Its seeds are
    /// checked to ensure it's a valid profile created by this program.
    #[account(
        mut,
        seeds = [b"admin", admin_profile.authority.as_ref()],
        bump
    )]
    pub admin_profile: Account<'info, AdminProfile>,
    /// The System Program, required for the lamport transfer from the user's PDA
    /// to the admin's PDA.
    pub system_program: Program<'info, System>,
}

/// Defines the accounts for the `log_action` instruction.
#[derive(Accounts)]
pub struct LogAction<'info> {
    /// The `Signer` of the transaction, who is the actor performing the action.
    /// This can be either a User's or an Admin's `ChainCard`.
    pub authority: Signer<'info>,
}
