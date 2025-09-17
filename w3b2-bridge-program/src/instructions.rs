use super::*;
use solana_program::program::invoke;
use solana_program::system_instruction;

fn register_pda<'info, T: AccountSerialize + AccountDeserialize + Clone>(
    pda_account: &mut Account<'info, T>,
    payer: &Signer<'info>,
    system_program: &Program<'info, System>,
    lamports: u64,
) -> Result<()> {
    let ix =
        system_instruction::transfer(&payer.key(), &pda_account.to_account_info().key, lamports);
    invoke(
        &ix,
        &[
            payer.to_account_info(),
            pda_account.to_account_info(),
            system_program.to_account_info(),
        ],
    )?;
    Ok(())
}

pub fn register_admin(
    ctx: Context<RegisterAdmin>,
    initial_balance: u64,
    communication_pubkey: Pubkey,
) -> Result<()> {
    let admin = &mut ctx.accounts.admin_account;
    admin.meta.owner = ctx.accounts.authority.key();
    admin.meta.co_signer = ctx.accounts.co_signer.key();
    admin.meta.communication_pubkey = communication_pubkey;
    admin.meta.active = true;

    // Получаем текущую стоимость аренды для аккаунта нашего размера
    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(8 + std::mem::size_of::<AdminAccount>());

    // Общая сумма, которую нужно положить на счет PDA
    let total_required_lamports = rent_exempt_minimum + initial_balance;

    msg!("Rent-exempt minimum: {} lamports", rent_exempt_minimum);
    msg!("Initial balance: {} lamports", initial_balance);
    msg!("Total required: {} lamports", total_required_lamports);

    // Проверяем, что у плательщика (payer) достаточно средств
    require!(
        ctx.accounts.payer.lamports() >= total_required_lamports,
        BridgeError::PayerInsufficientFunds
    );

    // Переводим на PDA всю необходимую сумму
    register_pda(
        admin,
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        total_required_lamports,
    )?;

    emit!(AdminRegistered {
        admin: ctx.accounts.authority.key(),
        initial_funding: initial_balance, // В событие отправляем только чистый баланс
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn register_user(
    ctx: Context<RegisterUser>,
    initial_balance: u64,
    communication_pubkey: Pubkey,
) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    user.meta.owner = ctx.accounts.user_wallet.key();
    user.meta.co_signer = ctx.accounts.co_signer.key();
    user.meta.communication_pubkey = communication_pubkey;
    user.meta.active = true;

    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(8 + std::mem::size_of::<UserAccount>());
    let total_required_lamports = rent_exempt_minimum + initial_balance;

    msg!("Rent-exempt minimum: {} lamports", rent_exempt_minimum);
    msg!("Initial balance: {} lamports", initial_balance);
    msg!("Total required: {} lamports", total_required_lamports);

    require!(
        ctx.accounts.payer.lamports() >= total_required_lamports,
        BridgeError::PayerInsufficientFunds
    );

    register_pda(
        user,
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        total_required_lamports,
    )?;

    emit!(UserRegistered {
        user: ctx.accounts.user_wallet.key(),
        initial_balance,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn deactivate_admin(ctx: Context<DeactivateAdmin>) -> Result<()> {
    let admin = &mut ctx.accounts.admin_account;
    admin.meta.deactivate();

    emit!(AdminDeactivated {
        admin: admin.meta.owner,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn deactivate_user(ctx: Context<DeactivateUser>) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    user.meta.deactivate();

    emit!(UserDeactivated {
        user: user.meta.owner,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn update_admin_comm_key(ctx: Context<UpdateAdminCommKey>, new_key: Pubkey) -> Result<()> {
    let admin_account = &mut ctx.accounts.admin_account;
    admin_account.meta.communication_pubkey = new_key;

    emit!(CommKeyUpdated {
        pda_owner: admin_account.meta.owner,
        new_comm_pubkey: new_key,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn update_user_comm_key(ctx: Context<UpdateUserCommKey>, new_key: Pubkey) -> Result<()> {
    let user_account = &mut ctx.accounts.user_account;
    user_account.meta.communication_pubkey = new_key;

    emit!(CommKeyUpdated {
        pda_owner: user_account.meta.owner,
        new_comm_pubkey: new_key,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn request_funding(
    ctx: Context<RequestFunding>,
    amount: u64,
    target_admin: Pubkey,
) -> Result<()> {
    let funding_request = &mut ctx.accounts.funding_request;
    funding_request.user_wallet = ctx.accounts.user_account.meta.owner;
    funding_request.amount = amount;
    funding_request.status = FundingStatus::Pending as u8;
    funding_request.target_admin = target_admin;

    emit!(FundingRequested {
        user_wallet: ctx.accounts.user_account.meta.owner,
        user_comm_pubkey: ctx.accounts.user_account.meta.communication_pubkey,
        target_admin,
        amount,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn approve_funding(ctx: Context<ApproveFunding>) -> Result<()> {
    let funding_request = &mut ctx.accounts.funding_request;
    let admin_account_info = ctx.accounts.admin_account.to_account_info();

    // Десериализуем данные админа, чтобы проверить статус
    let admin_data = AdminAccount::try_deserialize(&mut &admin_account_info.data.borrow()[..])?;
    require!(admin_data.meta.active, BridgeError::SenderInactive);

    require!(
        funding_request.target_admin == ctx.accounts.admin_authority.key(),
        BridgeError::Unauthorized
    );
    require!(
        funding_request.status == FundingStatus::Pending as u8,
        BridgeError::RequestAlreadyProcessed
    );

    // --- ГЛАВНАЯ ПРОВЕРКА RENT ---
    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(admin_account_info.data_len());
    let amount_to_transfer = funding_request.amount;

    // Проверяем, что после списания на счете админа останется достаточно средств для аренды
    require!(
        admin_account_info.lamports() - amount_to_transfer >= rent_exempt_minimum,
        BridgeError::AdminInsufficientFunds // ИСПОЛЬЗУЕМ КОНКРЕТНУЮ ОШИБКУ
    );

    // Безопасное изменение балансов
    **admin_account_info.try_borrow_mut_lamports()? -= amount_to_transfer;
    **ctx.accounts.user_wallet.try_borrow_mut_lamports()? += amount_to_transfer;

    funding_request.status = FundingStatus::Approved as u8;

    emit!(FundingApproved {
        user_wallet: funding_request.user_wallet,
        approved_by: funding_request.target_admin,
        admin_comm_pubkey: admin_data.meta.communication_pubkey,
        amount: amount_to_transfer,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn dispatch_command(
    ctx: Context<DispatchCommand>,
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
) -> Result<()> {
    require!(payload.len() <= 1024, BridgeError::PayloadTooLarge);

    let sender_info = &ctx.accounts.sender;
    let recipient_info = &ctx.accounts.recipient;
    let authority = &ctx.accounts.authority;
    let co_signer = &ctx.accounts.co_signer;

    if let Ok(user) = UserAccount::try_deserialize(&mut &sender_info.data.borrow()[..]) {
        require!(
            user.meta.owner == authority.key(),
            BridgeError::Unauthorized
        );
        require!(
            user.meta.co_signer == co_signer.key(),
            BridgeError::Unauthorized
        );
        require!(user.meta.active, BridgeError::SenderInactive);
    } else if let Ok(admin) = AdminAccount::try_deserialize(&mut &sender_info.data.borrow()[..]) {
        require!(
            admin.meta.owner == authority.key(),
            BridgeError::Unauthorized
        );
        require!(
            admin.meta.co_signer == co_signer.key(),
            BridgeError::Unauthorized
        );
        require!(admin.meta.active, BridgeError::SenderInactive);
    } else {
        return err!(BridgeError::InvalidAccountType);
    }

    if let Ok(user) = UserAccount::try_deserialize(&mut &recipient_info.data.borrow()[..]) {
        require!(user.meta.active, BridgeError::RecipientInactive);
    } else if let Ok(admin) = AdminAccount::try_deserialize(&mut &recipient_info.data.borrow()[..])
    {
        require!(admin.meta.active, BridgeError::RecipientInactive);
    } else {
        return err!(BridgeError::InvalidAccountType);
    }

    emit!(CommandEvent {
        sender: authority.key(),
        target: recipient_info.key(), // Используем ключ PDA получателя как цель
        command_id,
        mode,
        payload,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
    let actor_info = &ctx.accounts.actor;
    let authority = &ctx.accounts.authority;
    let co_signer = &ctx.accounts.co_signer;

    if let Ok(user) = UserAccount::try_deserialize(&mut &actor_info.data.borrow()[..]) {
        require!(
            user.meta.owner == authority.key(),
            BridgeError::Unauthorized
        );
        require!(
            user.meta.co_signer == co_signer.key(),
            BridgeError::Unauthorized
        );
    } else if let Ok(admin) = AdminAccount::try_deserialize(&mut &actor_info.data.borrow()[..]) {
        require!(
            admin.meta.owner == authority.key(),
            BridgeError::Unauthorized
        );
        require!(
            admin.meta.co_signer == co_signer.key(),
            BridgeError::Unauthorized
        );
    } else {
        return err!(BridgeError::InvalidAccountType);
    }

    emit!(HttpActionEvent {
        actor: authority.key(),
        session_id,
        action_code,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}
