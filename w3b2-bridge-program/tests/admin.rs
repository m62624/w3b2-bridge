mod instructions;

use crate::instructions::{admin, common};
use anchor_lang::AccountDeserialize;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::sysvar::rent::Rent;
use solana_sdk::signature::Signer;
use w3b2_bridge_program::state::AdminProfile;

#[test]
fn test_admin_create_profile_success() {
    // === 1. Arrange (Setup) ===

    // Initialize the Solana virtual machine and load our program.
    let mut svm = common::setup_svm();

    // Create a new keypair and fund it with 10 SOL. This keypair will act
    // as the `authority` for the new admin profile and pay for the transaction.
    let authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Create a separate keypair to simulate the off-chain communication key.
    let comm_key = common::create_keypair();

    // === 2. Act (Execution) ===

    println!("Attempting to create admin profile...");

    // Call our high-level helper function to create the profile.
    // This single call builds the instruction, sends the transaction, and returns the PDA.
    let admin_pda = admin::create_profile(&mut svm, &authority, comm_key.pubkey());

    println!("Admin profile created successfully at: {}", admin_pda);

    // === 3. Assert (Verification) ===

    // Fetch the newly created account's data from the SVM.
    let admin_account_data = svm.get_account(&admin_pda).unwrap();

    // Deserialize the raw account data into our `AdminProfile` struct.
    let admin_profile =
        AdminProfile::try_deserialize(&mut admin_account_data.data.as_slice()).unwrap();

    // Verify that the on-chain state was set correctly.
    assert_eq!(admin_profile.authority, authority.pubkey());
    assert_eq!(admin_profile.communication_pubkey, comm_key.pubkey());
    assert!(
        admin_profile.prices.is_empty(),
        "Prices vector should be empty on initialization"
    );
    assert_eq!(
        admin_profile.balance, 0,
        "Balance should be 0 on initialization"
    );

    // Verify that the account's lamport balance is exactly the rent-exempt minimum
    // for the space we allocated in the `AdminRegisterProfile` context.
    let rent = Rent::default();
    let space = 8 + std::mem::size_of::<AdminProfile>() + (10 * std::mem::size_of::<(u64, u64)>());
    let rent_exempt_minimum = rent.minimum_balance(space);

    assert_eq!(admin_account_data.lamports, rent_exempt_minimum);

    println!("✅ Assertions passed!");
    println!("   -> Authority: {}", admin_profile.authority);
    println!(
        "   -> PDA Lamports: {} (matches rent-exempt minimum)",
        admin_account_data.lamports
    );
}

#[test]
fn test_admin_update_comm_key_success() {
    // === 1. Arrange ===
    let mut svm = common::setup_svm();
    let authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Create the profile with an initial key.
    let initial_comm_key = common::create_keypair();
    let admin_pda = admin::create_profile(&mut svm, &authority, initial_comm_key.pubkey());

    // Define the new key we want to update to.
    let new_comm_key = common::create_keypair();

    // === 2. Act ===
    println!("Updating communication key...");

    // Call our new helper function to send the update transaction.
    admin::update_comm_key(&mut svm, &authority, new_comm_key.pubkey());

    // === 3. Assert ===

    // Fetch the account state AGAIN to see the changes.
    let admin_account_data = svm.get_account(&admin_pda).unwrap();
    let admin_profile =
        AdminProfile::try_deserialize(&mut admin_account_data.data.as_slice()).unwrap();

    // The main assertion: check if the key was updated.
    assert_eq!(admin_profile.communication_pubkey, new_comm_key.pubkey());

    // Also, ensure the old key is no longer there.
    assert_ne!(
        admin_profile.communication_pubkey,
        initial_comm_key.pubkey()
    );

    // Sanity check: ensure other fields were not changed.
    assert_eq!(admin_profile.authority, authority.pubkey());

    println!("✅ Update Comm Key Test Passed!");
    println!("   -> Old Key: {}", initial_comm_key.pubkey());
    println!("   -> New Key: {}", admin_profile.communication_pubkey);
}

#[test]
fn test_admin_close_profile_success() {
    // === 1. Arrange ===
    let mut svm = common::setup_svm();
    let authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let comm_key = common::create_keypair();

    // Create a profile that we can then close.
    let admin_pda = admin::create_profile(&mut svm, &authority, comm_key.pubkey());

    // Get the balances before the action.
    // The PDA balance is the rent-exempt minimum that should be returned.
    let pda_balance = svm.get_balance(&admin_pda).unwrap();
    let authority_balance_before = svm.get_balance(&authority.pubkey()).unwrap();

    // === 2. Act ===
    println!("Closing admin profile...");
    admin::close_profile(&mut svm, &authority);
    println!("Profile closed.");

    // === 3. Assert ===

    // Assertion 1: The admin profile account should no longer exist.
    let closed_account = svm.get_account(&admin_pda);
    assert!(closed_account.is_none(), "Account was not closed!");

    // Assertion 2: The authority's balance should have increased by the PDA's balance.
    // We also account for the transaction fee (5000 lamports in LiteSVM).
    let authority_balance_after = svm.get_balance(&authority.pubkey()).unwrap();
    let expected_balance = authority_balance_before + pda_balance - 5000;

    assert_eq!(authority_balance_after, expected_balance);

    println!("✅ Close Profile Test Passed!");
    println!(
        "   -> Authority balance correctly refunded: {} -> {}",
        authority_balance_before, authority_balance_after
    );
}

#[test]
fn test_admin_update_prices_success() {
    // === 1. Arrange ===
    let mut svm = common::setup_svm();
    let authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let comm_key = common::create_keypair();

    // Create a profile. Initially, its `prices` vector is empty.
    let admin_pda = admin::create_profile(&mut svm, &authority, comm_key.pubkey());

    // Define the new price list we want to set.
    let new_prices = vec![(1, 1000), (2, 2500), (5, 10000)];

    // Get the account size before the update to verify realloc works.
    let account_before = svm.get_account(&admin_pda).unwrap();
    let size_before = account_before.data.len();

    // === 2. Act ===
    println!("Updating prices for admin profile...");
    admin::update_prices(&mut svm, &authority, new_prices.clone());
    println!("Prices updated.");

    // === 3. Assert ===

    // Fetch the account state AGAIN to see the changes.
    let account_after = svm.get_account(&admin_pda).unwrap();
    let size_after = account_after.data.len();
    let admin_profile = AdminProfile::try_deserialize(&mut account_after.data.as_slice()).unwrap();

    // Assertion 1: The `prices` field should now contain our new data.
    assert_eq!(admin_profile.prices, new_prices);

    // Assertion 2: The account data size should have changed due to `realloc`.
    let base_size = 8 + std::mem::size_of::<AdminProfile>();
    let expected_size_after = base_size + (new_prices.len() * std::mem::size_of::<(u64, u64)>());

    assert_ne!(size_before, size_after, "Account size should have changed");
    assert_eq!(
        size_after, expected_size_after,
        "Account size is not what was expected after realloc"
    );

    println!("✅ Update Prices Test Passed!");
    println!("   -> Prices updated to: {:?}", admin_profile.prices);
    println!(
        "   -> Account size changed: {} -> {}",
        size_before, size_after
    );
}
