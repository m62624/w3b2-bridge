use super::*;

pub fn create_profile(
    svm: &mut LiteSVM,
    authority: &Keypair,
    comm_key: Pubkey,
    target_admin: Pubkey,
) -> Pubkey {
    let (create_ix, user_pda) = ix_create_profile(authority, comm_key, target_admin);
    build_and_send_tx(svm, vec![create_ix], authority, vec![]);
    user_pda
}

/// A high-level function that handles updating the communication key for a UserProfile.
pub fn update_comm_key(
    svm: &mut LiteSVM,
    authority: &Keypair,
    admin_pda: Pubkey,
    new_comm_key: Pubkey,
) {
    let update_ix = ix_update_comm_key(authority, admin_pda, new_comm_key);
    build_and_send_tx(svm, vec![update_ix], authority, vec![]);
}

/// A high-level function that handles closing a UserProfile.
pub fn close_profile(svm: &mut LiteSVM, authority: &Keypair, admin_pda: Pubkey) {
    let close_ix = ix_close_profile(authority, admin_pda);
    build_and_send_tx(svm, vec![close_ix], authority, vec![]);
}

/// Deposits lamports into a UserProfile PDA.
pub fn deposit(svm: &mut LiteSVM, authority: &Keypair, admin_pda: Pubkey, amount: u64) {
    let deposit_ix = ix_deposit(authority, admin_pda, amount);
    build_and_send_tx(svm, vec![deposit_ix], authority, vec![]);
}

/// Withdraws lamports from a user's UserProfile PDA to a destination wallet.
pub fn withdraw(
    svm: &mut LiteSVM,
    authority: &Keypair,
    admin_pda: Pubkey,
    destination: Pubkey,
    amount: u64,
) {
    let withdraw_ix = ix_withdraw(authority, admin_pda, destination, amount);
    build_and_send_tx(svm, vec![withdraw_ix], authority, vec![]);
}

// --- Low-level Instruction Builders ---

/// This function remains unchanged.
fn ix_create_profile(
    authority: &Keypair,
    communication_pubkey: Pubkey,
    target_admin: Pubkey,
) -> (Instruction, Pubkey) {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), target_admin.as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::UserCreateProfile {
        target_admin,
        communication_pubkey,
    }
    .data();

    let accounts = w3b2_accounts::UserCreateProfile {
        authority: authority.pubkey(),
        user_profile: user_pda,
        system_program: system_program::id(),
    }
    .to_account_metas(None);

    (
        Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts,
            data,
        },
        user_pda,
    )
}

fn ix_update_comm_key(authority: &Keypair, admin_pda: Pubkey, new_key: Pubkey) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_bridge_program::ID,
    );

    // Изменено: Убран `target_admin` из данных инструкции
    let data = w3b2_instruction::UserUpdateCommKey { new_key }.data();

    // Изменено: Добавлен `admin_pda` в аккаунты
    let accounts = w3b2_accounts::UserUpdateCommKey {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    }
}

fn ix_close_profile(authority: &Keypair, admin_pda: Pubkey) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_bridge_program::ID,
    );

    // Изменено: Данные инструкции теперь пустые
    let data = w3b2_instruction::UserCloseProfile {}.data();

    // Изменено: Добавлен `admin_pda` в аккаунты
    let accounts = w3b2_accounts::UserCloseProfile {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    }
}

fn ix_deposit(authority: &Keypair, admin_pda: Pubkey, amount: u64) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::UserDeposit { amount }.data();

    // Изменено: Добавлен `admin_pda` в аккаунты для безопасной проверки
    let accounts = w3b2_accounts::UserDeposit {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_pda,
        system_program: system_program::id(),
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    }
}

fn ix_withdraw(
    authority: &Keypair,
    admin_pda: Pubkey,
    destination: Pubkey,
    amount: u64,
) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_bridge_program::ID,
    );

    // Изменено: Убран `target_admin` из данных инструкции
    let data = w3b2_instruction::UserWithdraw { amount }.data();

    // Аккаунты уже были исправлены ранее, здесь все верно
    let accounts = w3b2_accounts::UserWithdraw {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_pda,
        destination,
        system_program: system_program::id(),
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    }
}
