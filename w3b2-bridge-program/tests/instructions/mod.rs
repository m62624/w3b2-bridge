use anchor_lang::{InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use solana_program::{instruction::Instruction, pubkey::Pubkey, system_program};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use w3b2_bridge_program::accounts as w3b2_accounts;
use w3b2_bridge_program::instruction as w3b2_instruction;

const PATH_SBF: &str = "../../target/deploy/w3b2_bridge_program.so";

/// Upload smart contract from file.
pub fn create_smart_contract() -> LiteSVM {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(w3b2_bridge_program::ID, PATH_SBF)
        .unwrap();
    svm
}

/// Creates a new random keypair.
/// This can be used for generating `owner`, `co_signer`, or any other required key.
pub fn create_keypair() -> Keypair {
    Keypair::new()
}

/// Creates a new "payer" keypair and funds it with SOL using the LiteSVM airdrop feature.
/// This keypair is used to pay for transaction fees and rent for new accounts.
///
/// # Arguments
/// * `svm` - A mutable reference to the LiteSVM instance.
/// * `lamports` - The amount of SOL (in lamports) to airdrop to the new keypair.
pub fn create_payer(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let keypair = Keypair::new();
    svm.airdrop(&keypair.pubkey(), lamports).unwrap();
    keypair
}

/// A helper function that builds and sends a transaction to the LiteSVM.
/// It wraps the provided instruction and signs it with all necessary keypairs.
///
/// # Arguments
/// * `svm` - A mutable reference to the LiteSVM instance.
/// * `ix` - The instruction to be executed.
/// * `payer` - The keypair that pays for the transaction.
/// * `signers` - A vector of additional keypairs required to sign the transaction.
pub fn build_and_send_tx(
    svm: &mut LiteSVM,
    ix: Instruction,
    payer: &Keypair,
    signers: Vec<&Keypair>,
) {
    // Create a new transaction with a compute budget instruction (good practice) and our main instruction.
    let mut tx = Transaction::new_with_payer(
        &[
            // It's recommended to add a compute budget instruction to avoid hitting default limits.
            ComputeBudgetInstruction::set_compute_unit_limit(200_000),
            ix,
        ],
        Some(&payer.pubkey()),
    );

    // Sign the transaction with the payer and all other required signers.
    tx.sign(&signers, svm.latest_blockhash());

    // Send the transaction to the SVM and assert that it succeeds.
    svm.send_transaction(tx).expect("Transaction failed");
}

/// A specific helper to construct the `register_admin` instruction.
/// This function encapsulates the logic of deriving the PDA and building the instruction struct.
///
/// # Arguments
/// * `payer` - The public key of the account funding the new PDA.
/// * `owner` - The keypair that will be the primary authority (`meta.owner`).
/// * `co_signer` - The keypair that will be the secondary authority (`meta.co_signer`).
/// * `communication_pubkey` - The public key for off-chain communication.
/// * `initial_balance` - The functional balance to add to the PDA.
pub fn ix_register_admin(
    payer: Pubkey,
    owner: &Keypair,
    co_signer: &Keypair,
    communication_pubkey: Pubkey,
    initial_balance: u64,
) -> (Instruction, Pubkey) {
    // Derive the Program-Derived Address (PDA) for the new admin account.
    // The seeds must match exactly what is defined in the on-chain program.
    let (admin_pda, _) = Pubkey::find_program_address(
        &[
            b"admin",
            owner.pubkey().as_ref(),
            co_signer.pubkey().as_ref(),
        ],
        &w3b2_bridge_program::ID,
    );

    // Construct the instruction data using the auto-generated struct from the program crate.
    // This is the modern and type-safe way to build instruction data with Anchor.
    let data = w3b2_instruction::RegisterAdmin {
        initial_balance,
        communication_pubkey,
    }
    .data();

    // Define the accounts required by the `RegisterAdmin` context.
    // The order must match the `#[derive(Accounts)]` struct in the program.
    let accounts = w3b2_accounts::RegisterAdmin {
        payer,
        authority: owner.pubkey(),
        co_signer: co_signer.pubkey(),
        admin_account: admin_pda,
        system_program: system_program::id(),
    }
    .to_account_metas(None); // `None` indicates this is not a CPI.

    // Assemble the final instruction.
    let ix = Instruction {
        program_id: w3b2_bridge_program::ID,
        accounts,
        data,
    };

    (ix, admin_pda)
}
