// tests/user.rs

mod instructions;

use crate::instructions::{admin, common, user};
use anchor_lang::AccountDeserialize;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::sysvar::rent::Rent;
use solana_sdk::signature::Signer;
use w3b2_bridge_program::state::UserProfile;

#[test]
fn test_user_create_profile_success() {
    // === 1. Arrange ===
    let mut svm = common::setup_svm();

    // We need an admin to exist first, which our user profile will link to.
    let admin_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(
        &mut svm,
        &admin_authority,
        common::create_keypair().pubkey(),
    );

    // Now, create the user that will interact with this admin.
    let user_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_comm_key = common::create_keypair();

    // === 2. Act ===
    println!("Attempting to create user profile...");

    // Call the user helper to create a profile, targeting the admin we just made.
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        user_comm_key.pubkey(),
        admin_pda, // <-- Link to the specific admin
    );

    println!("User profile created successfully at: {}", user_pda);

    // === 3. Assert ===

    // Fetch and deserialize the new user profile account.
    let user_account_data = svm.get_account(&user_pda).unwrap();
    let user_profile =
        UserProfile::try_deserialize(&mut user_account_data.data.as_slice()).unwrap();

    // Verify the on-chain state was set correctly.
    assert_eq!(user_profile.authority, user_authority.pubkey());
    assert_eq!(user_profile.communication_pubkey, user_comm_key.pubkey());
    assert_eq!(
        user_profile.deposit_balance, 0,
        "Deposit balance should be 0 on initialization"
    );

    // Verify the account's lamport balance is the rent-exempt minimum.
    let rent = Rent::default();
    let space = 8 + std::mem::size_of::<UserProfile>();
    let rent_exempt_minimum = rent.minimum_balance(space);

    assert_eq!(user_account_data.lamports, rent_exempt_minimum);

    println!("✅ Create User Profile Test Passed!");
    println!("   -> User Authority: {}", user_profile.authority);
    println!(
        "   -> PDA Lamports: {} (matches rent-exempt minimum)",
        user_account_data.lamports
    );
}

// In tests/user.rs

#[test]
fn test_user_update_comm_key_success() {
    // === 1. Arrange ===
    let mut svm = common::setup_svm();

    // Create an admin for the user to be linked to.
    let admin_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(
        &mut svm,
        &admin_authority,
        common::create_keypair().pubkey(),
    );

    // Create the user with an initial communication key.
    let user_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let initial_comm_key = common::create_keypair();
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        initial_comm_key.pubkey(),
        admin_pda,
    );

    // Define the new key we want to update to.
    let new_comm_key = common::create_keypair();

    // === 2. Act ===
    println!("Updating user communication key...");

    // Call our new helper function to send the update transaction.
    user::update_comm_key(&mut svm, &user_authority, admin_pda, new_comm_key.pubkey());

    // === 3. Assert ===

    // Fetch the account state AGAIN to see the changes.
    let user_account_data = svm.get_account(&user_pda).unwrap();
    let user_profile =
        UserProfile::try_deserialize(&mut user_account_data.data.as_slice()).unwrap();

    // The main assertion: check if the key was updated.
    assert_eq!(user_profile.communication_pubkey, new_comm_key.pubkey());

    // Also, ensure the old key is no longer there.
    assert_ne!(user_profile.communication_pubkey, initial_comm_key.pubkey());

    // Sanity check: ensure other fields were not changed.
    assert_eq!(user_profile.authority, user_authority.pubkey());
    assert_eq!(user_profile.deposit_balance, 0);

    println!("✅ Update User Comm Key Test Passed!");
    println!("   -> Old Key: {}", initial_comm_key.pubkey());
    println!("   -> New Key: {}", user_profile.communication_pubkey);
}

#[test]
fn test_user_close_profile_success() {
    // === 1. Arrange ===
    let mut svm = common::setup_svm();

    // Create an admin for the user to be linked to.
    let admin_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(
        &mut svm,
        &admin_authority,
        common::create_keypair().pubkey(),
    );

    // Create the user profile that we are going to close.
    let user_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        common::create_keypair().pubkey(),
        admin_pda,
    );

    // Get balances *after* creation but *before* closing.
    let pda_balance = svm.get_balance(&user_pda).unwrap();
    let authority_balance_before = svm.get_balance(&user_authority.pubkey()).unwrap();

    // === 2. Act ===
    println!("Closing user profile...");
    user::close_profile(&mut svm, &user_authority, admin_pda);
    println!("Profile closed.");

    // === 3. Assert ===

    // Assertion 1: The user profile account should no longer exist.
    let closed_account = svm.get_account(&user_pda);
    assert!(closed_account.is_none(), "Account was not closed!");

    // Assertion 2: The authority's balance should be refunded the rent money,
    // minus the transaction fee for the close operation (5000 lamports).
    let authority_balance_after = svm.get_balance(&user_authority.pubkey()).unwrap();
    let expected_balance = authority_balance_before + pda_balance - 5000;

    assert_eq!(authority_balance_after, expected_balance);

    println!("✅ Close User Profile Test Passed!");
    println!(
        "   -> User authority balance correctly refunded: {} -> {}",
        authority_balance_before, authority_balance_after
    );
}

// In tests/user.rs

#[test]
fn test_user_deposit_success() {
    // === 1. Arrange ===
    let mut svm = common::setup_svm();

    // Create an admin for the user to be linked to.
    let admin_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(
        &mut svm,
        &admin_authority,
        common::create_keypair().pubkey(),
    );

    // Create the user profile. Initially, its deposit_balance is 0.
    let user_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        common::create_keypair().pubkey(),
        admin_pda,
    );

    // Get the state *before* the deposit.
    let authority_balance_before = svm.get_balance(&user_authority.pubkey()).unwrap();
    let pda_lamports_before = svm.get_balance(&user_pda).unwrap();

    let deposit_amount = 2 * LAMPORTS_PER_SOL;

    // === 2. Act ===
    println!("User depositing {} lamports...", deposit_amount);

    // Call the deposit helper function.
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    println!("Deposit successful.");

    // === 3. Assert ===

    // Fetch the final state of the user profile and the authority's wallet.
    let user_account_data_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_data_after.data.as_slice()).unwrap();
    let authority_balance_after = svm.get_balance(&user_authority.pubkey()).unwrap();

    // Assertion 1: The internal `deposit_balance` field was correctly updated.
    assert_eq!(user_profile_after.deposit_balance, deposit_amount);

    // Assertion 2: The PDA's on-chain lamport balance correctly increased.
    assert_eq!(
        user_account_data_after.lamports,
        pda_lamports_before + deposit_amount
    );

    // Assertion 3: The authority's (ChainCard) balance correctly decreased.
    let expected_authority_balance = authority_balance_before - deposit_amount - 5000; // 5000 for tx fee
    assert_eq!(authority_balance_after, expected_authority_balance);

    println!("✅ User Deposit Test Passed!");
    println!(
        "   -> PDA internal balance is now: {}",
        user_profile_after.deposit_balance
    );
    println!(
        "   -> PDA lamport balance increased from {} to {}",
        pda_lamports_before, user_account_data_after.lamports
    );
}

#[test]
fn test_user_withdraw_success() {
    // === 1. Arrange ===
    let mut svm = common::setup_svm();

    // Create an admin for the user to be linked to.
    let admin_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(
        &mut svm,
        &admin_authority,
        common::create_keypair().pubkey(),
    );

    // Create the user profile.
    let user_authority = common::create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        common::create_keypair().pubkey(),
        admin_pda,
    );

    // The user deposits funds into their profile.
    let deposit_amount = 2 * LAMPORTS_PER_SOL;
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    // Create a destination wallet to receive the withdrawn funds.
    let destination_wallet = common::create_keypair();

    // Get the state *before* the withdrawal.
    let pda_lamports_before = svm.get_balance(&user_pda).unwrap();

    //
    // FIX: A new keypair has no on-chain account, so its balance is conceptually 0.
    //
    let destination_balance_before = 0; // <-- ИЗМЕНЕНИЕ ЗДЕСЬ

    let withdraw_amount = 1 * LAMPORTS_PER_SOL;

    // === 2. Act ===
    println!("User withdrawing {} lamports...", withdraw_amount);
    user::withdraw(
        &mut svm,
        &user_authority,
        admin_pda,
        destination_wallet.pubkey(),
        withdraw_amount,
    );
    println!("Withdrawal successful.");

    // === 3. Assert ===

    // Fetch the final state of the user profile and destination wallet.
    let user_account_data_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_data_after.data.as_slice()).unwrap();
    let destination_balance_after = svm.get_balance(&destination_wallet.pubkey()).unwrap();

    // Assertion 1: The internal `deposit_balance` was correctly updated.
    let expected_deposit_balance = deposit_amount - withdraw_amount;
    assert_eq!(user_profile_after.deposit_balance, expected_deposit_balance);

    // Assertion 2: The PDA's on-chain lamport balance correctly decreased.
    assert_eq!(
        user_account_data_after.lamports,
        pda_lamports_before - withdraw_amount
    );

    // Assertion 3: The destination wallet's balance correctly increased.
    assert_eq!(
        destination_balance_after,
        destination_balance_before + withdraw_amount
    );

    println!("✅ User Withdraw Test Passed!");
    println!(
        "   -> PDA internal balance is now: {}",
        user_profile_after.deposit_balance
    );
    println!(
        "   -> Destination wallet received: {} lamports",
        destination_balance_after
    );
}
