// tests/instructions/user.rs

use super::*;

// --- High-Level Helper Functions ---

/// A high-level test helper that orchestrates the creation of a `UserProfile`.
///
/// This function builds the `user_create_profile` instruction, sends it in a transaction
/// signed by the user's `authority`, and returns the address of the newly created PDA.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The `Keypair` of the user's `ChainCard`, who will own the new profile.
/// * `comm_key` - The `Pubkey` to be set as the initial off-chain communication key.
/// * `target_admin` - The `Pubkey` of the `AdminProfile` PDA this new profile will be linked to.
///
/// # Returns
/// The `Pubkey` of the newly created `UserProfile` PDA.
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

/// A high-level test helper that updates the communication key for an existing `UserProfile`.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The user's `ChainCard` `Keypair`, which must be the owner of the profile.
/// * `admin_pda` - The `Pubkey` of the `AdminProfile` the user is associated with. This is
///   required to derive the correct `UserProfile` PDA address.
/// * `new_comm_key` - The new `Pubkey` to set as the communication key.
pub fn update_comm_key(
    svm: &mut LiteSVM,
    authority: &Keypair,
    admin_pda: Pubkey,
    new_comm_key: Pubkey,
) {
    let update_ix = ix_update_comm_key(authority, admin_pda, new_comm_key);
    build_and_send_tx(svm, vec![update_ix], authority, vec![]);
}

/// A high-level test helper that closes a `UserProfile` account.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The user's `ChainCard` `Keypair`, who must own the profile.
///   This keypair will also receive the rent refund from the closed account.
/// * `admin_pda` - The `Pubkey` of the `AdminProfile` the user is associated with,
///   required to find the correct `UserProfile` PDA to close.
pub fn close_profile(svm: &mut LiteSVM, authority: &Keypair, admin_pda: Pubkey) {
    let close_ix = ix_close_profile(authority, admin_pda);
    build_and_send_tx(svm, vec![close_ix], authority, vec![]);
}

/// A high-level test helper that deposits lamports into a `UserProfile` PDA.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The user's `ChainCard` `Keypair`.
/// * `admin_pda` - The `Pubkey` of the `AdminProfile` the user is associated with.
/// * `amount` - The amount of lamports to deposit.
pub fn deposit(svm: &mut LiteSVM, authority: &Keypair, admin_pda: Pubkey, amount: u64) {
    let deposit_ix = ix_deposit(authority, admin_pda, amount);
    build_and_send_tx(svm, vec![deposit_ix], authority, vec![]);
}

/// A high-level test helper that withdraws lamports from a `UserProfile`'s deposit balance.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The user's `ChainCard` `Keypair`.
/// * `admin_pda` - The `Pubkey` of the `AdminProfile` the user is associated with.
/// * `destination` - The `Pubkey` of the wallet that will receive the withdrawn lamports.
/// * `amount` - The amount of lamports to withdraw.
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

/// A high-level test helper that allows a user to send a command to a service.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The user's `ChainCard` `Keypair`, who is initiating the command.
/// * `admin_pda` - The `Pubkey` of the target `AdminProfile` service.
/// * `command_id` - The `u64` identifier for the command.
/// * `payload` - A `Vec<u8>` containing arbitrary data for the command.
pub fn dispatch_command(
    svm: &mut LiteSVM,
    authority: &Keypair,
    admin_pda: Pubkey,
    command_id: u64,
    payload: Vec<u8>,
) {
    let dispatch_ix = ix_dispatch_command(authority, admin_pda, command_id, payload);
    build_and_send_tx(svm, vec![dispatch_ix], authority, vec![]);
}

// --- Low-Level Instruction Builders ---

/// A low-level builder for the `user_create_profile` instruction.
/// It derives the `UserProfile` PDA from the user's `authority` and the target `AdminProfile` PDA,
/// then constructs the instruction `data` and `accounts` contexts.
///
/// # Returns
/// A tuple containing the configured `Instruction` and the `Pubkey` of the `user_pda`.
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

/// A low-level builder for the `user_update_comm_key` instruction.
fn ix_update_comm_key(authority: &Keypair, admin_pda: Pubkey, new_key: Pubkey) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::UserUpdateCommKey { new_key }.data();

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

/// A low-level builder for the `user_close_profile` instruction.
fn ix_close_profile(authority: &Keypair, admin_pda: Pubkey) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::UserCloseProfile {}.data();

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

/// A low-level builder for the `user_deposit` instruction.
fn ix_deposit(authority: &Keypair, admin_pda: Pubkey, amount: u64) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::UserDeposit { amount }.data();

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

/// A low-level builder for the `user_withdraw` instruction.
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

    let data = w3b2_instruction::UserWithdraw { amount }.data();

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

/// A low-level builder for the `user_dispatch_command` instruction.
fn ix_dispatch_command(
    authority: &Keypair,
    admin_pda: Pubkey,
    command_id: u64,
    payload: Vec<u8>,
) -> Instruction {
    let (user_pda, _) = Pubkey::find_program_address(
        &[b"user", authority.pubkey().as_ref(), admin_pda.as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::UserDispatchCommand {
        command_id,
        payload,
    }
    .data();

    let accounts = w3b2_accounts::UserDispatchCommand {
        authority: authority.pubkey(),
        user_profile: user_pda,
        admin_profile: admin_pda,
        system_program: system_program::id(),
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    }
}
