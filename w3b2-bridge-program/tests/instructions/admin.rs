use super::*;
use w3b2_bridge_program::state::{PriceEntry, UpdatePricesArgs};

// --- High-Level Helper Functions ---

/// A high-level test helper that orchestrates the creation of an `AdminProfile`.
///
/// This function builds the `admin_register_profile` instruction, sends it in a transaction
/// signed by the `authority`, and returns the address of the newly created PDA.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The `Keypair` of the admin's `ChainCard`, who will own the new profile.
/// * `comm_key` - The `Pubkey` to be set as the initial off-chain communication key.
///
/// # Returns
/// The `Pubkey` of the newly created `AdminProfile` PDA.
pub fn create_profile(svm: &mut LiteSVM, authority: &Keypair, comm_key: Pubkey) -> Pubkey {
    let (register_ix, admin_pda) = ix_create_profile(authority, comm_key);
    build_and_send_tx(svm, vec![register_ix], authority, vec![]);
    admin_pda
}

/// A high-level test helper that updates the communication key for an existing `AdminProfile`.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The admin's `ChainCard` `Keypair`, which must be the owner of the profile.
/// * `new_comm_key` - The new `Pubkey` to set as the communication key.
pub fn update_comm_key(svm: &mut LiteSVM, authority: &Keypair, new_comm_key: Pubkey) {
    let update_ix = ix_update_comm_key(authority, new_comm_key);
    build_and_send_tx(svm, vec![update_ix], authority, vec![]);
}

/// A high-level test helper that closes an `AdminProfile` account.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The admin's `ChainCard` `Keypair`, who must own the profile.
///   This keypair will also receive the rent refund from the closed account.
pub fn close_profile(svm: &mut LiteSVM, authority: &Keypair) {
    let close_ix = ix_close_profile(authority);
    build_and_send_tx(svm, vec![close_ix], authority, vec![]);
}

/// A high-level test helper that updates the price list for an `AdminProfile`.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The admin's `ChainCard` `Keypair`.
/// * `new_prices` - A vector of `(u64, u64)` tuples representing the new price list.
pub fn update_prices(svm: &mut LiteSVM, authority: &Keypair, new_prices: Vec<PriceEntry>) {
    let update_ix = ix_update_prices(authority, new_prices);
    build_and_send_tx(svm, vec![update_ix], authority, vec![]);
}

/// A high-level test helper that withdraws earned funds from an `AdminProfile`.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The admin's `ChainCard` `Keypair`.
/// * `destination` - The `Pubkey` of the wallet that will receive the withdrawn lamports.
/// * `amount` - The amount of lamports to withdraw.
pub fn withdraw(svm: &mut LiteSVM, authority: &Keypair, destination: Pubkey, amount: u64) {
    let withdraw_ix = ix_withdraw(authority, destination, amount);
    build_and_send_tx(svm, vec![withdraw_ix], authority, vec![]);
}

/// A high-level test helper that allows an admin to send a command to a user.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `authority` - The admin's `ChainCard` `Keypair`, who is initiating the command.
/// * `user_profile_pda` - The `Pubkey` of the target `UserProfile` account.
/// * `command_id` - The `u64` identifier for the command.
/// * `payload` - A `Vec<u8>` containing arbitrary data for the command.
pub fn dispatch_command(
    svm: &mut LiteSVM,
    authority: &Keypair,
    user_profile_pda: Pubkey,
    command_id: u64,
    payload: Vec<u8>,
) {
    let dispatch_ix = ix_dispatch_command(authority, user_profile_pda, command_id, payload);
    build_and_send_tx(svm, vec![dispatch_ix], authority, vec![]);
}

// --- Low-Level Instruction Builders ---

/// A low-level builder for the `admin_register_profile` instruction.
///
/// It derives the `AdminProfile` PDA, then constructs the instruction `data` and
/// `accounts` contexts before assembling the final `Instruction`.
///
/// # Returns
/// A tuple containing the configured `Instruction` and the `Pubkey` of the `admin_pda`.
fn ix_create_profile(authority: &Keypair, communication_pubkey: Pubkey) -> (Instruction, Pubkey) {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::AdminRegisterProfile {
        communication_pubkey,
    }
    .data();

    let accounts = w3b2_accounts::AdminRegisterProfile {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
        system_program: system_program::id(),
    }
    .to_account_metas(None);

    let ix = Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    };

    (ix, admin_pda)
}

/// A low-level builder for the `admin_update_comm_key` instruction.
fn ix_update_comm_key(authority: &Keypair, new_key: Pubkey) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::AdminUpdateCommKey { new_key }.data();

    let accounts = w3b2_accounts::AdminUpdateCommKey {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    }
}

/// A low-level builder for the `admin_close_profile` instruction.
fn ix_close_profile(authority: &Keypair) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::AdminCloseProfile {}.data();

    let accounts = w3b2_accounts::AdminCloseProfile {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    }
}

/// A low-level builder for the `admin_update_prices` instruction.
fn ix_update_prices(authority: &Keypair, new_prices: Vec<PriceEntry>) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_bridge_program::ID,
    );

    let args = UpdatePricesArgs { new_prices };
    let data = w3b2_instruction::AdminUpdatePrices { args }.data();

    let accounts = w3b2_accounts::AdminUpdatePrices {
        authority: authority.pubkey(),
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

/// A low-level builder for the `admin_withdraw` instruction.
fn ix_withdraw(authority: &Keypair, destination: Pubkey, amount: u64) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::AdminWithdraw { amount }.data();

    let accounts = w3b2_accounts::AdminWithdraw {
        authority: authority.pubkey(),
        admin_profile: admin_pda,
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

/// A low-level builder for the `admin_dispatch_command` instruction.
fn ix_dispatch_command(
    authority: &Keypair,
    user_profile_pda: Pubkey,
    command_id: u64,
    payload: Vec<u8>,
) -> Instruction {
    let (admin_pda, _) = Pubkey::find_program_address(
        &[b"admin", authority.pubkey().as_ref()],
        &w3b2_bridge_program::ID,
    );

    let data = w3b2_instruction::AdminDispatchCommand {
        command_id,
        payload,
    }
    .data();

    let accounts = w3b2_accounts::AdminDispatchCommand {
        admin_authority: authority.pubkey(),
        admin_profile: admin_pda,
        user_profile: user_profile_pda,
    }
    .to_account_metas(None);

    Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    }
}
