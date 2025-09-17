use super::*;

#[account]
#[derive(Debug)]
pub struct AccountMeta {
    pub owner: Pubkey,
    pub co_signer: Pubkey,
    pub communication_pubkey: Pubkey,
    pub active: bool,
}

impl AccountMeta {
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

#[account]
#[derive(Debug)]
pub struct AdminAccount {
    pub meta: AccountMeta,
}

#[account]
#[derive(Debug)]
pub struct UserAccount {
    pub meta: AccountMeta,
}

#[derive(Accounts)]
pub struct RegisterAdmin<'info> {
    #[account(mut)]
    pub payer: Signer<'info>, // Ключ №1: плательщик
    pub authority: Signer<'info>, // Ключ №2: основной
    pub co_signer: Signer<'info>, // Ключ №3: дополнительный
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<AdminAccount>(),
        seeds = [b"admin", authority.key().as_ref(), co_signer.key().as_ref()],
        bump
    )]
    pub admin_account: Account<'info, AdminAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterUser<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub user_wallet: Signer<'info>, // Для юзера это и есть authority
    pub co_signer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<UserAccount>(),
        seeds = [b"user", user_wallet.key().as_ref(), co_signer.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DeactivateAdmin<'info> {
    pub authority: Signer<'info>, // Ключ №2
    pub co_signer: Signer<'info>, // Ключ №3
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref(), co_signer.key().as_ref()],
        bump,
        constraint = admin_account.meta.owner == authority.key() @ BridgeError::Unauthorized,
        constraint = admin_account.meta.co_signer == co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub admin_account: Account<'info, AdminAccount>,
}

#[derive(Accounts)]
pub struct UpdateAdminCommKey<'info> {
    pub authority: Signer<'info>,
    pub co_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"admin", authority.key().as_ref(), co_signer.key().as_ref()],
        bump,
        constraint = admin_account.meta.owner == authority.key() @ BridgeError::Unauthorized,
        constraint = admin_account.meta.co_signer == co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub admin_account: Account<'info, AdminAccount>,
}

#[derive(Accounts)]
pub struct UpdateUserCommKey<'info> {
    // Для юзера authority - это его user_wallet
    pub authority: Signer<'info>,
    pub co_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), co_signer.key().as_ref()],
        bump,
        constraint = user_account.meta.owner == authority.key() @ BridgeError::Unauthorized,
        constraint = user_account.meta.co_signer == co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub user_account: Account<'info, UserAccount>,
}

#[derive(Accounts)]
pub struct DeactivateUser<'info> {
    pub authority: Signer<'info>,
    pub co_signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user", authority.key().as_ref(), co_signer.key().as_ref()],
        bump,
        constraint = user_account.meta.owner == authority.key() @ BridgeError::Unauthorized,
        constraint = user_account.meta.co_signer == co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub user_account: Account<'info, UserAccount>,
}

/// Funding request storage
#[account]
#[derive(Debug)]
pub struct FundingRequest {
    pub user_wallet: Pubkey,
    pub target_admin: Pubkey,
    pub amount: u64,
    pub status: u8,
}

#[derive(Debug, Accounts)]
pub struct RequestFunding<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 8 + 1,
        seeds = [b"funding", user_account.key().as_ref(), &payer.key().to_bytes()],
        bump
    )]
    pub funding_request: Account<'info, FundingRequest>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub user_account: Account<'info, UserAccount>,
    pub system_program: Program<'info, System>,
}

/// Admin approves funding request
#[derive(Accounts)]
pub struct ApproveFunding<'info> {
    pub admin_authority: Signer<'info>, // Ключ №2 админа
    pub admin_co_signer: Signer<'info>, // Ключ №3 админа
    #[account(
        mut,
        seeds = [b"admin", admin_authority.key().as_ref(), admin_co_signer.key().as_ref()],
        bump,
        constraint = admin_account.meta.owner == admin_authority.key() @ BridgeError::Unauthorized,
        constraint = admin_account.meta.co_signer == admin_co_signer.key() @ BridgeError::Unauthorized,
    )]
    pub admin_account: Account<'info, AdminAccount>,
    #[account(mut)]
    pub funding_request: Account<'info, FundingRequest>,
    #[account(mut)]
    pub user_wallet: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DispatchCommand<'info> {
    /// Ключ №2 отправителя (его `owner`).
    pub authority: Signer<'info>,
    /// Ключ №3 отправителя.
    pub co_signer: Signer<'info>,

    /// Аккаунт отправителя (может быть UserAccount или AdminAccount).
    /// Проверяем, что владелец аккаунта - наша программа.
    #[account(
        mut,
        constraint = sender.owner == &crate::ID @ BridgeError::InvalidAccountOwner
    )]
    pub sender: AccountInfo<'info>,

    /// Аккаунт получателя. Также должен принадлежать нашей программе.
    #[account(
        mut,
        constraint = recipient.owner == &crate::ID @ BridgeError::InvalidAccountOwner
    )]
    pub recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct LogAction<'info> {
    pub authority: Signer<'info>,
    pub co_signer: Signer<'info>,
    #[account(
        constraint = actor.owner == &crate::ID @ BridgeError::InvalidAccountOwner
    )]
    pub actor: AccountInfo<'info>, // Аккаунт того, кто действует (User или Admin)
}
