use super::*;

//
// Account State Structures
// These structs define the shape of the data stored on-chain in Solana accounts.
//

/// `AccountMeta` holds the core, shared metadata for both Admin and User PDAs.
/// This centralized structure ensures consistency and simplifies management.
#[account]
#[derive(Debug)]
pub struct AccountMeta {
    /// KEY #2: The primary on-chain authority for this PDA.
    /// This public key is required as a signer for most sensitive instructions
    /// that modify this account's state. It represents the main controller of the account.
    pub owner: Pubkey,

    /// KEY #3: The secondary on-chain authority for this PDA, enabling multi-sig.
    /// This public key is also required as a signer alongside the `owner`,
    /// providing an additional layer of security.
    pub co_signer: Pubkey,

    /// The public key designated for off-chain communication and encryption (e.g., X25519).
    /// The corresponding private key is kept securely off-chain and is used to decrypt
    /// payloads, such as session keys in `CommandConfig`. This key is stored on-chain
    /// so that other parties can use it to encrypt data for this account.
    pub communication_pubkey: Pubkey,

    /// A boolean flag indicating whether the account is currently active and can be used.
    /// If `false`, most instructions involving this account will fail.
    pub active: bool,
}

impl AccountMeta {
    /// Sets the account's status to inactive.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// The main Program-Derived Address (PDA) for an administrator (a service provider).
/// It encapsulates the metadata and acts as the on-chain identity for a service.
#[account]
#[derive(Debug)]
pub struct AdminAccount {
    /// The shared metadata for this admin account.
    pub meta: AccountMeta,
}

/// The main Program-Derived Address (PDA) for a user (a client of a service).
/// It encapsulates the metadata and acts as the on-chain identity for a user.
#[account]
#[derive(Debug)]
pub struct UserAccount {
    /// The shared metadata for this user account.
    pub meta: AccountMeta,
}

//
// Instruction Contexts
// These structs define the accounts required by each instruction. Anchor uses them
// to enforce security checks and manage account access automatically.
//

/// Context for the `register_admin` instruction.
#[derive(Accounts)]
pub struct RegisterAdmin<'info> {
    /// KEY #1: The account that pays the lamports (rent) for the new PDA's creation.
    /// This is typically a temporary or external wallet. Must be a signer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// KEY #2: The main authority for the new Admin PDA. Must be a signer.
    /// This key will be stored in `admin_account.meta.owner`.
    pub authority: Signer<'info>,

    /// KEY #3: The secondary authority for the new Admin PDA. Must be a signer.
    /// This key will be stored in `admin_account.meta.co_signer`.
    pub co_signer: Signer<'info>,

    /// The new `AdminAccount` PDA to be created.
    #[account(
        init, // Specifies that this account will be initialized.
        payer = payer, // Designates the `payer` account to fund the creation.
        space = 8 + std::mem::size_of::<AdminAccount>(), // Allocates necessary space (8 bytes for discriminator + size of struct).
        seeds = [b"admin", authority.key().as_ref(), co_signer.key().as_ref()], // PDA seeds for deterministic address derivation.
        bump // The canonical bump seed for this PDA.
    )]
    pub admin_account: Account<'info, AdminAccount>,

    /// A required account for any instruction that creates a new account.
    pub system_program: Program<'info, System>,
}

/// Context for the `register_user` instruction.
#[derive(Accounts)]
pub struct RegisterUser<'info> {
    /// KEY #1: The account paying for the new User PDA's rent. Must be a signer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// KEY #2: The main authority for the new User PDA. For users, this is their primary wallet. Must be a signer.
    /// It will be stored in `user_account.meta.owner`.
    pub user_wallet: Signer<'info>,

    /// KEY #3: The secondary authority for the new User PDA. Must be a signer.
    pub co_signer: Signer<'info>,

    /// The new `UserAccount` PDA to be created.
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<UserAccount>(),
        seeds = [b"user", user_wallet.key().as_ref(), co_signer.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,

    /// The Solana System Program, required for account creation.
    pub system_program: Program<'info, System>,
}

/// Context for the `deactivate_admin` instruction. Enforces multi-sig.
#[derive(Accounts)]
pub struct DeactivateAdmin<'info> {
    /// The `owner` of the AdminAccount. Must be a signer.
    pub authority: Signer<'info>,
    /// The `co_signer` of the AdminAccount. Must be a signer.
    pub co_signer: Signer<'info>,
    /// The AdminAccount PDA to be deactivated.
    #[account(
        mut, // The account's data will be modified (meta.active = false).
        seeds = [b"admin", authority.key().as_ref(), co_signer.key().as_ref()], // Ensures the PDA matches the signers.
        bump,
        // Security check: Verifies that the `owner` field stored on-chain matches the `authority` signer.
        constraint = admin_account.meta.owner == authority.key() @ BridgeError::Unauthorized,
        // Security check: Verifies that the `co_signer` field stored on-chain matches the `co_signer` signer.
        constraint = admin_account.meta.co_signer == co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub admin_account: Account<'info, AdminAccount>,
}

/// Context for the `update_admin_comm_key` instruction. Enforces multi-sig.
#[derive(Accounts)]
pub struct UpdateAdminCommKey<'info> {
    /// The `owner` of the AdminAccount. Must be a signer.
    pub authority: Signer<'info>,
    /// The `co_signer` of the AdminAccount. Must be a signer.
    pub co_signer: Signer<'info>,
    /// The AdminAccount PDA whose communication key will be updated.
    #[account(
        mut, // The account's data will be modified.
        seeds = [b"admin", authority.key().as_ref(), co_signer.key().as_ref()],
        bump,
        constraint = admin_account.meta.owner == authority.key() @ BridgeError::Unauthorized,
        constraint = admin_account.meta.co_signer == co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub admin_account: Account<'info, AdminAccount>,
}

/// Context for the `update_user_comm_key` instruction. Enforces multi-sig.
#[derive(Accounts)]
pub struct UpdateUserCommKey<'info> {
    /// The `owner` of the UserAccount. Must be a signer.
    pub authority: Signer<'info>,
    /// The `co_signer` of the UserAccount. Must be a signer.
    pub co_signer: Signer<'info>,
    /// The UserAccount PDA whose communication key will be updated.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), co_signer.key().as_ref()],
        bump,
        constraint = user_account.meta.owner == authority.key() @ BridgeError::Unauthorized,
        constraint = user_account.meta.co_signer == co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub user_account: Account<'info, UserAccount>,
}

/// Context for the `deactivate_user` instruction. Enforces multi-sig.
#[derive(Accounts)]
pub struct DeactivateUser<'info> {
    /// The `owner` of the UserAccount. Must be a signer.
    pub authority: Signer<'info>,
    /// The `co_signer` of the UserAccount. Must be a signer.
    pub co_signer: Signer<'info>,
    /// The UserAccount PDA to be deactivated.
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), co_signer.key().as_ref()],
        bump,
        constraint = user_account.meta.owner == authority.key() @ BridgeError::Unauthorized,
        constraint = user_account.meta.co_signer == co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub user_account: Account<'info, UserAccount>,
}

/// On-chain account to store the state of a funding request.
#[account]
#[derive(Debug)]
pub struct FundingRequest {
    /// The `owner` of the user account requesting funds.
    pub user_wallet: Pubkey,
    /// The `owner` of the admin account being targeted.
    pub target_admin: Pubkey,
    /// The requested amount in lamports.
    pub amount: u64,
    /// The current status of the request (Pending, Approved, Rejected).
    pub status: u8,
}

/// Context for the `request_funding` instruction.
#[derive(Debug, Accounts)]
pub struct RequestFunding<'info> {
    /// The new `FundingRequest` PDA to be created.
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<FundingRequest>(),
        seeds = [b"funding", user_account.key().as_ref(), &payer.key().to_bytes()], // Seeds include user PDA and the payer to ensure uniqueness.
        bump
    )]
    pub funding_request: Account<'info, FundingRequest>,

    /// The account paying for the `FundingRequest` PDA's rent. Can be the user's main wallet.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The UserAccount PDA that is making the request. Used to derive seeds and retrieve metadata.
    pub user_account: Account<'info, UserAccount>,

    /// The Solana System Program.
    pub system_program: Program<'info, System>,
}

/// Context for the `approve_funding` instruction.
#[derive(Accounts)]
pub struct ApproveFunding<'info> {
    /// The `owner` of the AdminAccount. Must be a signer.
    pub admin_authority: Signer<'info>,
    /// The `co_signer` of the AdminAccount. Must be a signer.
    pub admin_co_signer: Signer<'info>,
    /// The AdminAccount PDA that will fund the request. Its balance will be debited.
    #[account(
        mut,
        seeds = [b"admin", admin_authority.key().as_ref(), admin_co_signer.key().as_ref()],
        bump,
        constraint = admin_account.meta.owner == admin_authority.key() @ BridgeError::Unauthorized,
        constraint = admin_account.meta.co_signer == admin_co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub admin_account: Account<'info, AdminAccount>,

    /// The `FundingRequest` PDA to be processed. Its status will be updated.
    #[account(mut)]
    pub funding_request: Account<'info, FundingRequest>,

    /// CHECK:
    /// The target user's wallet (`owner` of the User PDA) to receive the funds.
    /// Marked as `AccountInfo` and unchecked because we are only transferring lamports to it,
    /// not reading or writing its data. The address is verified against `funding_request`.
    #[account(mut)]
    pub user_wallet: AccountInfo<'info>,
}

/// Universal context for the bi-directional `dispatch_command` instruction.
#[derive(Accounts)]
pub struct DispatchCommand<'info> {
    /// The `owner` of the sending account (sender). Must be a signer.
    pub authority: Signer<'info>,
    /// The `co_signer` of the sending account (sender). Must be a signer.
    pub co_signer: Signer<'info>,

    /// CHECK: The sender's PDA (either `UserAccount` or `AdminAccount`).
    /// This is the correct pattern when an account can be one of multiple types.
    #[account(
        mut,
        constraint = sender.owner == &crate::ID @ BridgeError::InvalidAccountOwner
    )]
    pub sender: AccountInfo<'info>,

    /// CHECK: The recipient's PDA (either `UserAccount` or `AdminAccount`).
    #[account(
        mut,
        constraint = recipient.owner == &crate::ID @ BridgeError::InvalidAccountOwner
    )]
    pub recipient: AccountInfo<'info>,
}

/// Context for the `log_action` instruction.
#[derive(Accounts)]
pub struct LogAction<'info> {
    /// The `owner` of the account performing the off-chain action. Must be a signer.
    pub authority: Signer<'info>,
    /// The `co_signer` of the account performing the action. Must be a signer.
    pub co_signer: Signer<'info>,

    /// The PDA (`UserAccount` or `AdminAccount`) of the entity performing the action.
    /// It's a generic `AccountInfo` whose ownership is verified by the signers.
    /// CHECK:
    #[account(
        constraint = actor.owner == &crate::ID @ BridgeError::InvalidAccountOwner
    )]
    pub actor: AccountInfo<'info>,
}
