// tests/instructions/mod.rs

/// This module contains high-level test helper functions for Admin-related instructions.
pub mod admin;
/// This module contains high-level test helper functions for User-related instructions.
pub mod user;

use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use solana_program::{instruction::Instruction, pubkey::Pubkey, system_program};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use w3b2_bridge_program::{accounts as w3b2_accounts, instruction as w3b2_instruction};

/// A constant path to the compiled on-chain program binary (`.so` file).
/// This is used by `setup_svm` to load the program into the test environment.
const PATH_SBF: &str = "../target/deploy/w3b2_bridge_program.so";

/// Initializes the `LiteSVM` test environment and loads the W3B2 Bridge program into it.
/// This function serves as the foundation for every test case, creating a fresh,
/// sandboxed "virtual blockchain" for each test to run in.
///
/// # Returns
/// A new instance of `LiteSVM` with the program successfully loaded.
pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(w3b2_bridge_program::ID, PATH_SBF)
        .unwrap();
    svm
}

/// A simple wrapper for `Keypair::new()` for consistency across tests.
///
/// # Returns
/// A new, randomly generated `Keypair`.
pub fn create_keypair() -> Keypair {
    Keypair::new()
}

/// Creates a new `Keypair` and funds its on-chain account with a specified amount of lamports.
/// This is essential for creating `authority` or `payer` accounts (`ChainCards`) that need
/// to sign transactions and pay for fees and rent.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment where the airdrop will occur.
/// * `lamports` - The amount of lamports to airdrop to the new keypair's account.
///
/// # Returns
/// The new, funded `Keypair`.
pub fn create_funded_keypair(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports).unwrap();
    keypair
}

/// A generic helper to construct, sign, and send a transaction to the `LiteSVM`.
/// This function is the workhorse for executing instructions in the test environment.
/// It automatically includes a `ComputeBudget` instruction and handles signing from
/// multiple keypairs.
///
/// # Arguments
/// * `svm` - A mutable reference to the `LiteSVM` test environment.
/// * `instructions` - A vector of `Instruction`s to be included in the transaction.
/// * `payer_and_signer` - The primary `Keypair` that will both sign the transaction
///   and pay for the associated fees. This typically represents a User's or Admin's `ChainCard`.
/// * `additional_signers` - A vector of other `Keypair`s that are required to sign
///   the transaction, if any.
pub fn build_and_send_tx(
    svm: &mut LiteSVM,
    instructions: Vec<Instruction>,
    payer_and_signer: &Keypair,
    additional_signers: Vec<&Keypair>,
) {
    let mut signers = vec![payer_and_signer];
    signers.extend(additional_signers);

    // Prepend a compute budget instruction to prevent transaction failures on complex instructions.
    let mut all_instructions = vec![ComputeBudgetInstruction::set_compute_unit_limit(400_000)];
    all_instructions.extend(instructions);

    let mut tx = Transaction::new_with_payer(&all_instructions, Some(&payer_and_signer.pubkey()));

    tx.sign(&signers, svm.latest_blockhash());

    // Send the transaction and panic if it fails, providing immediate feedback in the test run.
    svm.send_transaction(tx).expect("Transaction failed");
}
