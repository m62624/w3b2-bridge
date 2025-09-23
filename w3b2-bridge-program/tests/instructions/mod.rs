pub mod admin;
pub mod user;

use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use solana_program::{instruction::Instruction, pubkey::Pubkey, system_program};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use w3b2_bridge_program::{accounts as w3b2_accounts, instruction as w3b2_instruction};

// Use a constant for the path to the compiled program binary.
const PATH_SBF: &str = "../target/deploy/w3b2_bridge_program.so";

/// Loads the compiled smart contract into a new LiteSVM simulator instance.
pub fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(w3b2_bridge_program::ID, PATH_SBF)
        .unwrap();
    svm
}

/// Creates a new, random Keypair.
pub fn create_keypair() -> Keypair {
    Keypair::new()
}

/// Creates a new Keypair and funds its account with lamports via airdrop.
/// Useful for creating `authority` accounts that need to pay for transactions.
pub fn create_funded_keypair(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports).unwrap();
    keypair
}

/// A generic function to build and send a transaction to the SVM.
pub fn build_and_send_tx(
    svm: &mut LiteSVM,
    instructions: Vec<Instruction>,
    // The primary signer who also pays for the transaction fees.
    payer_and_signer: &Keypair,
    // Any other signers required by the instruction(s).
    additional_signers: Vec<&Keypair>,
) {
    let mut signers = vec![payer_and_signer];
    signers.extend(additional_signers);

    let mut all_instructions = vec![ComputeBudgetInstruction::set_compute_unit_limit(400_000)];
    all_instructions.extend(instructions);

    let mut tx = Transaction::new_with_payer(&all_instructions, Some(&payer_and_signer.pubkey()));

    tx.sign(&signers, svm.latest_blockhash());

    svm.send_transaction(tx).expect("Transaction failed");
}
