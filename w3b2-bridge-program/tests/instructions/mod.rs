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

/// Common helper functions that can be used across all tests.
pub mod common {
    use super::*;

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

        let mut tx =
            Transaction::new_with_payer(&all_instructions, Some(&payer_and_signer.pubkey()));

        tx.sign(&signers, svm.latest_blockhash());

        svm.send_transaction(tx).expect("Transaction failed");
    }
}

/// Helper functions specific to Admin actions.
pub mod admin {
    use w3b2_bridge_program::state::UpdatePricesArgs;

    use super::*;

    /// A high-level function that handles the complete creation of an AdminProfile.
    /// It builds the instruction, sends the transaction, and returns the new PDA's address.
    pub fn create_profile(svm: &mut LiteSVM, authority: &Keypair, comm_key: Pubkey) -> Pubkey {
        // Build the instruction required to register the admin profile.
        let (register_ix, admin_pda) = ix_create_profile(authority, comm_key);

        // Send the instruction in a transaction, signed by the authority.
        common::build_and_send_tx(svm, vec![register_ix], authority, vec![]);

        // Return the address of the newly created PDA for assertions in the test.
        admin_pda
    }

    /// Updates the communication key for an existing AdminProfile.
    pub fn update_comm_key(svm: &mut LiteSVM, authority: &Keypair, new_comm_key: Pubkey) {
        let update_ix = ix_update_comm_key(authority, new_comm_key);
        common::build_and_send_tx(svm, vec![update_ix], authority, vec![]);
    }

    /// A high-level function that handles closing an AdminProfile.
    pub fn close_profile(svm: &mut LiteSVM, authority: &Keypair) {
        // Build the instruction required to close the admin profile.
        let close_ix = ix_close_profile(authority);

        // Send the instruction in a transaction.
        common::build_and_send_tx(svm, vec![close_ix], authority, vec![]);
    }

    pub fn update_prices(svm: &mut LiteSVM, authority: &Keypair, new_prices: Vec<(u64, u64)>) {
        let update_ix = ix_update_prices(authority, new_prices);
        common::build_and_send_tx(svm, vec![update_ix], authority, vec![]);
    }

    /// A low-level helper to build the `admin_register_profile` instruction.
    fn ix_create_profile(
        authority: &Keypair,
        communication_pubkey: Pubkey,
    ) -> (Instruction, Pubkey) {
        // Derive the Program-Derived Address (PDA) for the new admin profile.
        // The seeds must exactly match the ones defined in the on-chain program.
        let (admin_pda, _) = Pubkey::find_program_address(
            &[b"admin", authority.pubkey().as_ref()],
            &w3b2_bridge_program::ID,
        );

        // Construct the instruction data using the auto-generated struct from the `instruction` module.
        let data = w3b2_instruction::AdminRegisterProfile {
            communication_pubkey,
        }
        .data();

        // Construct the list of accounts required by the `AdminRegisterProfile` context,
        // using the auto-generated struct from the `accounts` module.
        let accounts = w3b2_accounts::AdminRegisterProfile {
            authority: authority.pubkey(),
            admin_profile: admin_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None);

        // Assemble the final instruction.
        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts,
            data,
        };

        (ix, admin_pda)
    }

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

    /// A low-level helper to build the `admin_close_profile` instruction.
    fn ix_close_profile(authority: &Keypair) -> Instruction {
        // Find the PDA address for the profile to be closed.
        let (admin_pda, _) = Pubkey::find_program_address(
            &[b"admin", authority.pubkey().as_ref()],
            &w3b2_bridge_program::ID,
        );

        // This instruction has no arguments, so its data is empty.
        let data = w3b2_instruction::AdminCloseProfile {}.data();

        // The accounts context requires the authority and the profile to close.
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

    /// A low-level helper to build the `admin_update_prices` instruction.
    fn ix_update_prices(authority: &Keypair, new_prices: Vec<(u64, u64)>) -> Instruction {
        let (admin_pda, _) = Pubkey::find_program_address(
            &[b"admin", authority.pubkey().as_ref()],
            &w3b2_bridge_program::ID,
        );

        // Create the argument container struct.
        let args = UpdatePricesArgs { new_prices };

        // Build the instruction data.
        let data = w3b2_instruction::AdminUpdatePrices { args }.data();

        // The accounts context requires authority, the profile, and system_program for realloc.
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
}
