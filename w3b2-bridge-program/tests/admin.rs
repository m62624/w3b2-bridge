// mod instructions;
// use anchor_lang::AccountDeserialize;
// use solana_program::native_token::LAMPORTS_PER_SOL;
// use solana_program::sysvar::rent::Rent;
// use solana_sdk::signature::Signer;

// use instructions::*;

// #[test]
// fn test_register_admin_success() {
//     // --- 1. Setup ---
//     let mut svm = create_smart_contract();

//     // Create and fund the `payer` account, which will pay for the transaction fees and rent.
//     let payer = create_payer(&mut svm, 10 * LAMPORTS_PER_SOL); // Airdrop 10 SOL

//     // Create the keypairs that will act as the authorities for the new Admin PDA.
//     let owner = create_keypair();
//     let co_signer = create_keypair();

//     // Create a keypair that will be used for off-chain communication.
//     // In a real scenario, this would be a carefully managed X25519 key.
//     let comm_key = create_keypair();

//     // Define the initial functional balance for the new PDA.
//     let initial_balance = 1 * LAMPORTS_PER_SOL; // 1 SOL

//     // --- 2. Action ---
//     // Build the `register_admin` instruction using our helper.
//     let (ix, admin_pda) = ix_register_admin(
//         payer.pubkey(),
//         &owner,
//         &co_signer,
//         comm_key.pubkey(),
//         initial_balance,
//     );

//     // Build and send the transaction to the SVM.
//     // The signers must include the payer and any key specified as `Signer` in the context.
//     // In `RegisterAdmin`, `payer`, `authority` (owner), and `co_signer` are all signers.
//     build_and_send_tx(&mut svm, ix, &payer, vec![&payer, &owner, &co_signer]);

//     // --- 3. Assert ---
//     // Fetch the newly created admin account from the SVM.
//     let admin_account_data = svm.get_account(&admin_pda).unwrap();

//     // Deserialize the account's binary data into the `AdminAccount` struct.
//     let admin_account =
//         AdminAccount::try_deserialize(&mut admin_account_data.data.as_slice()).unwrap();

//     // Verify that all metadata fields were set correctly.
//     assert_eq!(admin_account.meta.owner, owner.pubkey());
//     assert_eq!(admin_account.meta.co_signer, co_signer.pubkey());
//     assert_eq!(admin_account.meta.communication_pubkey, comm_key.pubkey());
//     assert_eq!(admin_account.meta.active, true);

//     // Verify that the PDA's balance is correct.
//     // It should be the rent-exempt minimum for its size plus the initial balance we provided.
//     let rent = Rent::default();
//     let rent_exempt_minimum = rent.minimum_balance(std::mem::size_of::<AdminAccount>() + 8);
//     assert_eq!(
//         admin_account_data.lamports,
//         rent_exempt_minimum + initial_balance
//     );

//     println!("✅ Admin PDA created successfully at: {}", admin_pda);
//     println!(
//         "✅ PDA Balance: {} lamports (Rent: {}, Initial: {})",
//         admin_account_data.lamports, rent_exempt_minimum, initial_balance
//     );
// }
