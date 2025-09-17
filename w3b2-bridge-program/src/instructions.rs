use super::*;
// Import necessary modules from the Solana Program crate.
// `invoke` is used to call another program's instructions (in this case, the System Program).
// `system_instruction` provides helpers for creating instructions like `transfer`.
use solana_program::program::invoke;
use solana_program::system_instruction;

/// A generic helper function to create a Program-Derived Address (PDA) and fund it.
/// This is used by both `register_admin` and `register_user`.
///
/// # Arguments
/// * `pda_account` - The account to be created and funded.
/// * `payer` - The account that will pay the lamports for the PDA's rent and initial balance.
/// * `system_program` - A reference to the Solana System Program, required for creating accounts and transferring lamports.
/// * `lamports` - The total amount of lamports to transfer to the new PDA.
fn register_pda<'info, T: AccountSerialize + AccountDeserialize + Clone>(
    pda_account: &mut Account<'info, T>,
    payer: &Signer<'info>,
    system_program: &Program<'info, System>,
    lamports: u64,
) -> Result<()> {
    // Create a new lamport transfer instruction from the `payer` to the `pda_account`.
    let ix =
        system_instruction::transfer(&payer.key(), &pda_account.to_account_info().key, lamports);

    // Invoke the System Program to execute the transfer instruction.
    // This is a Cross-Program Invocation (CPI).
    invoke(
        &ix,
        // The accounts required by the `transfer` instruction.
        &[
            payer.to_account_info(),
            pda_account.to_account_info(),
            system_program.to_account_info(),
        ],
    )?;
    Ok(())
}

/// Instruction to register a new Admin PDA.
///
/// # Arguments
/// * `ctx` - The context holding all required accounts for this instruction.
/// * `initial_balance` - The functional balance to be added on top of the rent-exempt minimum.
/// * `communication_pubkey` - The off-chain public key for encryption purposes.
pub fn register_admin(
    ctx: Context<RegisterAdmin>,
    initial_balance: u64,
    communication_pubkey: Pubkey,
) -> Result<()> {
    // Get a mutable reference to the newly initialized admin_account.
    let admin = &mut ctx.accounts.admin_account;
    // Populate the metadata for the admin account from the context's signers.
    admin.meta.owner = ctx.accounts.authority.key();
    admin.meta.co_signer = ctx.accounts.co_signer.key();
    admin.meta.communication_pubkey = communication_pubkey;
    admin.meta.active = true;

    // --- Rent Calculation and Funding ---
    // Retrieve the Rent sysvar to get the current rent-exempt minimum.
    // This makes the logic robust against future changes in Solana's rent fees.
    let rent = Rent::get()?;
    // Calculate the minimum balance required for the account to be rent-exempt, based on its size.
    let rent_exempt_minimum = rent.minimum_balance(8 + std::mem::size_of::<AdminAccount>());

    // Calculate the total amount required: the rent + the desired usable balance.
    let total_required_lamports = rent_exempt_minimum + initial_balance;

    // Log the calculated values for easier debugging from the client-side.
    msg!("Rent-exempt minimum: {} lamports", rent_exempt_minimum);
    msg!("Initial balance: {} lamports", initial_balance);
    msg!("Total required: {} lamports", total_required_lamports);

    // Security Check: Ensure the payer has enough funds to cover the total required amount.
    require!(
        ctx.accounts.payer.lamports() >= total_required_lamports,
        BridgeError::PayerInsufficientFunds
    );

    // Call the helper function to perform the actual lamport transfer.
    register_pda(
        admin,
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        total_required_lamports,
    )?;

    // Emit an event to notify off-chain listeners that a new admin has been registered.
    emit!(AdminRegistered {
        admin: ctx.accounts.authority.key(),
        initial_funding: initial_balance, // The event reports the functional balance, not the total.
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Instruction to register a new User PDA.
/// The logic is nearly identical to `register_admin`.
pub fn register_user(
    ctx: Context<RegisterUser>,
    initial_balance: u64,
    communication_pubkey: Pubkey,
) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    // For users, the `user_wallet` signer acts as the `owner`.
    user.meta.owner = ctx.accounts.user_wallet.key();
    user.meta.co_signer = ctx.accounts.co_signer.key();
    user.meta.communication_pubkey = communication_pubkey;
    user.meta.active = true;

    // Calculate the rent-exempt minimum for the UserAccount struct.
    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(8 + std::mem::size_of::<UserAccount>());
    let total_required_lamports = rent_exempt_minimum + initial_balance;

    msg!("Rent-exempt minimum: {} lamports", rent_exempt_minimum);
    msg!("Initial balance: {} lamports", initial_balance);
    msg!("Total required: {} lamports", total_required_lamports);

    // Security Check: Ensure the payer has sufficient funds.
    require!(
        ctx.accounts.payer.lamports() >= total_required_lamports,
        BridgeError::PayerInsufficientFunds
    );

    // Transfer the total required lamports to the new User PDA.
    register_pda(
        user,
        &ctx.accounts.payer,
        &ctx.accounts.system_program,
        total_required_lamports,
    )?;

    // Emit an event to notify off-chain listeners.
    emit!(UserRegistered {
        user: ctx.accounts.user_wallet.key(),
        initial_balance,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Deactivates an Admin PDA. Requires multi-sig.
pub fn deactivate_admin(ctx: Context<DeactivateAdmin>) -> Result<()> {
    let admin = &mut ctx.accounts.admin_account;
    // Call the helper method on the metadata to set the `active` flag to false.
    admin.meta.deactivate();

    // Emit an event to log this action.
    emit!(AdminDeactivated {
        admin: admin.meta.owner,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Deactivates a User PDA. Requires multi-sig.
pub fn deactivate_user(ctx: Context<DeactivateUser>) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    user.meta.deactivate();

    emit!(UserDeactivated {
        user: user.meta.owner,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Updates the communication public key for an Admin PDA. Requires multi-sig.
/// # Arguments
/// * `new_key` - The new public key to be used for off-chain communication.
pub fn update_admin_comm_key(ctx: Context<UpdateAdminCommKey>, new_key: Pubkey) -> Result<()> {
    let admin_account = &mut ctx.accounts.admin_account;
    // Directly update the `communication_pubkey` field.
    admin_account.meta.communication_pubkey = new_key;

    // Emit a generic event to log that a key was updated.
    emit!(CommKeyUpdated {
        pda_owner: admin_account.meta.owner,
        new_comm_pubkey: new_key,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Updates the communication public key for a User PDA. Requires multi-sig.
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

/// Creates a new `FundingRequest` PDA.
/// This instruction is called by a user to request funds from an admin.
pub fn request_funding(
    ctx: Context<RequestFunding>,
    amount: u64,
    target_admin: Pubkey,
) -> Result<()> {
    // Populate the fields of the newly created FundingRequest account.
    let funding_request = &mut ctx.accounts.funding_request;
    funding_request.user_wallet = ctx.accounts.user_account.meta.owner;
    funding_request.amount = amount;
    funding_request.status = FundingStatus::Pending as u8; // Set initial status to Pending.
    funding_request.target_admin = target_admin;

    // Emit an event so the targeted admin's off-chain service can see the request.
    // Include the user's communication key so the admin can prepare for a connection.
    emit!(FundingRequested {
        user_wallet: ctx.accounts.user_account.meta.owner,
        user_comm_pubkey: ctx.accounts.user_account.meta.communication_pubkey,
        target_admin,
        amount,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// Approves a funding request and transfers lamports from the Admin PDA to the user's wallet.
pub fn approve_funding(ctx: Context<ApproveFunding>) -> Result<()> {
    let funding_request = &mut ctx.accounts.funding_request;
    let admin_account_info = ctx.accounts.admin_account.to_account_info();

    // Manually deserialize the admin account data to access its fields for checks.
    // This is necessary because the account constraints are handled by the context.
    let admin_data = AdminAccount::try_deserialize(&mut &admin_account_info.data.borrow()[..])?;

    // --- Security and State Checks ---
    // 1. Ensure the admin account approving the request is active.
    require!(admin_data.meta.active, BridgeError::SenderInactive);

    // 2. Ensure the admin signing this transaction is the one targeted by the request.
    require!(
        funding_request.target_admin == ctx.accounts.admin_authority.key(),
        BridgeError::Unauthorized
    );
    // 3. Prevent double-spending by ensuring the request is still pending.
    require!(
        funding_request.status == FundingStatus::Pending as u8,
        BridgeError::RequestAlreadyProcessed
    );

    // --- Rent-Exempt Balance Check ---
    let rent = Rent::get()?;
    let rent_exempt_minimum = rent.minimum_balance(admin_account_info.data_len());
    let amount_to_transfer = funding_request.amount;

    // CRITICAL: Ensure the transfer will not cause the admin PDA's balance
    // to fall below the rent-exempt minimum, which could risk the account being garbage collected.
    require!(
        admin_account_info.lamports() - amount_to_transfer >= rent_exempt_minimum,
        BridgeError::AdminInsufficientFunds
    );

    // --- Lamport Transfer ---
    // Safely borrow and modify the lamport balances of the two accounts.
    // This is a direct, low-level way to transfer funds between accounts within a program.
    **admin_account_info.try_borrow_mut_lamports()? -= amount_to_transfer;
    **ctx.accounts.user_wallet.try_borrow_mut_lamports()? += amount_to_transfer;

    // Update the request status to prevent it from being processed again.
    funding_request.status = FundingStatus::Approved as u8;

    // Emit an event to notify the user that their request was approved.
    // Include the admin's communication key so the user can initiate an encrypted connection.
    emit!(FundingApproved {
        user_wallet: funding_request.user_wallet,
        approved_by: funding_request.target_admin,
        admin_comm_pubkey: admin_data.meta.communication_pubkey,
        amount: amount_to_transfer,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// A universal, bi-directional instruction for sending commands between User and Admin PDAs.
/// The `payload` is an opaque byte array, allowing for flexible off-chain logic.
pub fn dispatch_command(
    ctx: Context<DispatchCommand>,
    command_id: u64,
    mode: CommandMode,
    payload: Vec<u8>,
) -> Result<()> {
    // Enforce a maximum payload size to prevent excessive transaction sizes and costs.
    require!(payload.len() <= 1024, BridgeError::PayloadTooLarge);

    let sender_info = &ctx.accounts.sender;
    let recipient_info = &ctx.accounts.recipient;
    let authority = &ctx.accounts.authority;
    let co_signer = &ctx.accounts.co_signer;

    // --- Sender Verification ---
    // Since `sender` is a generic `AccountInfo`, we must manually deserialize its data
    // to determine if it's a User or Admin and then verify its state and authorities.
    if let Ok(user) = UserAccount::try_deserialize(&mut &sender_info.data.borrow()[..]) {
        // Verify that the transaction signers match the `owner` and `co_signer` stored in the sender's PDA.
        require!(
            user.meta.owner == authority.key(),
            BridgeError::Unauthorized
        );
        require!(
            user.meta.co_signer == co_signer.key(),
            BridgeError::Unauthorized
        );
        // Ensure the sender's account is active.
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
        // If the account is neither a UserAccount nor an AdminAccount, it's an invalid type.
        return err!(BridgeError::InvalidAccountType);
    }

    // --- Recipient Verification ---
    // Also verify that the recipient account is active before sending a command.
    if let Ok(user) = UserAccount::try_deserialize(&mut &recipient_info.data.borrow()[..]) {
        require!(user.meta.active, BridgeError::RecipientInactive);
    } else if let Ok(admin) = AdminAccount::try_deserialize(&mut &recipient_info.data.borrow()[..])
    {
        require!(admin.meta.active, BridgeError::RecipientInactive);
    } else {
        return err!(BridgeError::InvalidAccountType);
    }

    // Emit the command as an event for off-chain services to consume.
    emit!(CommandEvent {
        sender: authority.key(),
        target: recipient_info.key(), // The target is the recipient's PDA key.
        command_id,
        mode,
        payload,
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}

/// A lightweight instruction to log a significant off-chain action on the blockchain.
/// This creates an immutable audit trail for interactions that happened over the direct HTTP channel.
pub fn log_action(ctx: Context<LogAction>, session_id: u64, action_code: u16) -> Result<()> {
    let actor_info = &ctx.accounts.actor;
    let authority = &ctx.accounts.authority;
    let co_signer = &ctx.accounts.co_signer;

    // Verify the identity of the actor logging the action by checking the multi-sig pair.
    // This ensures that only the legitimate owner of the PDA can log actions on its behalf.
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

    // Emit the event containing the details of the off-chain action.
    emit!(HttpActionEvent {
        actor: authority.key(), // The `owner` of the PDA that performed the action.
        session_id,             // Links this log to an initial `dispatch_command` session.
        action_code, // A numeric code representing the specific action (e.g., 10=Download).
        ts: clock::Clock::get()?.unix_timestamp,
    });

    Ok(())
}
