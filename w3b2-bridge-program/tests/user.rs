//! This module contains all integration tests for User-related instructions.
//!
//! The tests follow a standard Arrange-Act-Assert pattern:
//! 1.  **Arrange:** Set up the initial on-chain state (create admins, users, fund wallets).
//! 2.  **Act:** Execute the single instruction being tested.
//! 3.  **Assert:** Fetch the resulting on-chain state and verify that it matches the expected outcome.

mod instructions;

use anchor_lang::AccountDeserialize;
use instructions::*;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::sysvar::rent::Rent;
use solana_sdk::signature::Signer;
use w3b2_bridge_program::state::{AdminProfile, PriceEntry, UserProfile};

/// Tests the successful creation of a `UserProfile` PDA.
///
/// ### Scenario
/// A new user wants to register with a service (Admin) that already exists on the protocol.
///
/// ### Arrange
/// 1. An `AdminProfile` is created.
/// 2. A new `Keypair` is created and funded to act as the user's `ChainCard` (`user_authority`).
/// 3. A `Keypair` is created for the user's off-chain communication key.
///
/// ### Act
/// The `user::create_profile` helper is called, creating the on-chain `UserProfile` PDA.
///
/// ### Assert
/// 1. The `authority` field in the new `UserProfile` matches the user's `ChainCard` public key.
/// 2. The `communication_pubkey` field is set correctly.
/// 3. The initial `deposit_balance` is 0.
/// 4. The account's lamport balance is exactly the rent-exempt minimum for its size.
#[test]
fn test_user_create_profile_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    // We need an admin to exist first, which our user profile will link to.
    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    // Now, create the user that will interact with this admin.
    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_comm_key = create_keypair();

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
    let user_account_data = svm.get_account(&user_pda).unwrap();
    let user_profile =
        UserProfile::try_deserialize(&mut user_account_data.data.as_slice()).unwrap();

    assert_eq!(user_profile.authority, user_authority.pubkey());
    assert_eq!(user_profile.communication_pubkey, user_comm_key.pubkey());
    assert_eq!(
        user_profile.deposit_balance, 0,
        "Deposit balance should be 0 on initialization"
    );

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

/// Tests the successful update of a `UserProfile`'s communication key.
///
/// ### Scenario
/// A user with an existing profile wants to change their key for off-chain communication.
///
/// ### Arrange
/// 1. An `AdminProfile` and a `UserProfile` are created with an initial communication key.
/// 2. A new `Keypair` is generated for the new communication key.
///
/// ### Act
/// The `user::update_comm_key` helper is called.
///
/// ### Assert
/// 1. The `communication_pubkey` field in the `UserProfile` is updated to the new key.
/// 2. The other fields (`authority`, `deposit_balance`) remain unchanged.
#[test]
fn test_user_update_comm_key_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let initial_comm_key = create_keypair();
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        initial_comm_key.pubkey(),
        admin_pda,
    );

    let new_comm_key = create_keypair();

    // === 2. Act ===
    println!("Updating user communication key...");
    user::update_comm_key(&mut svm, &user_authority, admin_pda, new_comm_key.pubkey());

    // === 3. Assert ===
    let user_account_data = svm.get_account(&user_pda).unwrap();
    let user_profile =
        UserProfile::try_deserialize(&mut user_account_data.data.as_slice()).unwrap();

    assert_eq!(user_profile.communication_pubkey, new_comm_key.pubkey());
    assert_ne!(user_profile.communication_pubkey, initial_comm_key.pubkey());
    assert_eq!(user_profile.authority, user_authority.pubkey());
    assert_eq!(user_profile.deposit_balance, 0);

    println!("✅ Update User Comm Key Test Passed!");
    println!("   -> Old Key: {}", initial_comm_key.pubkey());
    println!("   -> New Key: {}", user_profile.communication_pubkey);
}

/// Tests the successful closure of a `UserProfile` account.
///
/// ### Scenario
/// A user decides to stop using a service and closes their profile to recover the rent lamports.
///
/// ### Arrange
/// 1. An `AdminProfile` and `UserProfile` are created.
/// 2. The lamport balances of the user's `ChainCard` and the `UserProfile` PDA are recorded.
///
/// ### Act
/// The `user::close_profile` helper is called.
///
/// ### Assert
/// 1. The `UserProfile` PDA account no longer exists.
/// 2. The balance of the user's `ChainCard` (`authority`) has increased by the lamport
///    balance of the closed PDA, minus the transaction fee.
#[test]
fn test_user_close_profile_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    let pda_balance = svm.get_balance(&user_pda).unwrap();
    let authority_balance_before = svm.get_balance(&user_authority.pubkey()).unwrap();

    // === 2. Act ===
    println!("Closing user profile...");
    user::close_profile(&mut svm, &user_authority, admin_pda);
    println!("Profile closed.");

    // === 3. Assert ===
    let closed_account = svm.get_account(&user_pda);
    assert!(closed_account.is_none(), "Account was not closed!");

    let authority_balance_after = svm.get_balance(&user_authority.pubkey()).unwrap();
    let expected_balance = authority_balance_before + pda_balance - 5000;
    assert_eq!(authority_balance_after, expected_balance);

    println!("✅ Close User Profile Test Passed!");
    println!(
        "   -> User authority balance correctly refunded: {} -> {}",
        authority_balance_before, authority_balance_after
    );
}

/// Tests the successful deposit of funds into a `UserProfile`.
///
/// ### Scenario
/// A user pre-funds their profile to pay for future services from an admin.
///
/// ### Arrange
/// 1. An `AdminProfile` and `UserProfile` are created.
/// 2. The balances of the user's `ChainCard` and the `UserProfile` PDA are recorded.
///
/// ### Act
/// The `user::deposit` helper is called to transfer lamports.
///
/// ### Assert
/// 1. The `deposit_balance` field inside the `UserProfile` is correctly incremented.
/// 2. The on-chain lamport balance of the `UserProfile` PDA increases by the deposit amount.
/// 3. The balance of the user's `ChainCard` (`authority`) decreases by the deposit amount, plus the transaction fee.
#[test]
fn test_user_deposit_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    let authority_balance_before = svm.get_balance(&user_authority.pubkey()).unwrap();
    let pda_lamports_before = svm.get_balance(&user_pda).unwrap();

    let deposit_amount = 2 * LAMPORTS_PER_SOL;

    // === 2. Act ===
    println!("User depositing {} lamports...", deposit_amount);
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);
    println!("Deposit successful.");

    // === 3. Assert ===
    let user_account_data_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_data_after.data.as_slice()).unwrap();
    let authority_balance_after = svm.get_balance(&user_authority.pubkey()).unwrap();

    assert_eq!(user_profile_after.deposit_balance, deposit_amount);

    assert_eq!(
        user_account_data_after.lamports,
        pda_lamports_before + deposit_amount
    );

    let expected_authority_balance = authority_balance_before - deposit_amount - 5000;
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

/// Tests the successful withdrawal of funds from a `UserProfile`.
///
/// ### Scenario
/// A user withdraws their unspent deposit from a service profile to a different wallet.
///
/// ### Arrange
/// 1. An `AdminProfile` and `UserProfile` are created.
/// 2. The user deposits funds into their profile.
/// 3. A new `destination_wallet` is created.
///
/// ### Act
/// The `user::withdraw` helper is called.
///
/// ### Assert
/// 1. The `deposit_balance` field in the `UserProfile` is correctly decremented.
/// 2. The on-chain lamport balance of the `UserProfile` PDA decreases by the withdrawal amount.
/// 3. The lamport balance of the `destination_wallet` increases by the withdrawal amount.
#[test]
fn test_user_withdraw_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    let deposit_amount = 2 * LAMPORTS_PER_SOL;
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    let destination_wallet = create_keypair();

    let pda_lamports_before = svm.get_balance(&user_pda).unwrap();
    let destination_balance_before = 0;
    let withdraw_amount = LAMPORTS_PER_SOL;

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
    let user_account_data_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_data_after.data.as_slice()).unwrap();
    let destination_balance_after = svm.get_balance(&destination_wallet.pubkey()).unwrap();

    let expected_deposit_balance = deposit_amount - withdraw_amount;
    assert_eq!(user_profile_after.deposit_balance, expected_deposit_balance);

    assert_eq!(
        user_account_data_after.lamports,
        pda_lamports_before - withdraw_amount
    );

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

/// Tests the successful execution of a paid command from a user to an admin.
///
/// ### Scenario
/// This is the primary use case of the protocol. A user pays a service (Admin) for an
/// off-chain action by calling the `user_dispatch_command` instruction.
///
/// ### Arrange
/// 1. An `AdminProfile` is created.
/// 2. The Admin sets a price for a specific `command_id`.
/// 3. A `UserProfile` is created and linked to the admin.
/// 4. The user deposits enough funds to cover the command price.
/// 5. The initial state of both the user and admin profiles are recorded.
///
/// ### Act
/// The `user::dispatch_command` helper is called.
///
/// ### Assert
/// 1. The user's `deposit_balance` and on-chain lamports decrease by the command price.
/// 2. The admin's `balance` and on-chain lamports increase by the command price.
#[test]
fn test_user_dispatch_command_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());
    let command_id_to_call = 1;
    let command_price = LAMPORTS_PER_SOL;
    admin::update_prices(
        &mut svm,
        &admin_authority,
        vec![PriceEntry::new(command_id_to_call, command_price)],
    );

    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );
    let deposit_amount = 2 * LAMPORTS_PER_SOL;
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    let user_pda_lamports_before = svm.get_balance(&user_pda).unwrap();
    let admin_pda_lamports_before = svm.get_balance(&admin_pda).unwrap();

    let admin_account_data_before = svm.get_account(&admin_pda).unwrap();
    let admin_profile_before =
        AdminProfile::try_deserialize(&mut admin_account_data_before.data.as_slice()).unwrap();

    // === 2. Act ===
    println!("User dispatching paid command...");
    user::dispatch_command(
        &mut svm,
        &user_authority,
        admin_pda,
        command_id_to_call,
        vec![1, 2, 3], // Arbitrary payload
    );
    println!("Command dispatched successfully.");

    // === 3. Assert ===
    let user_account_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_after.data.as_slice()).unwrap();

    let admin_account_after = svm.get_account(&admin_pda).unwrap();
    let admin_profile_after =
        AdminProfile::try_deserialize(&mut admin_account_after.data.as_slice()).unwrap();

    // Assert user balances
    assert_eq!(
        user_profile_after.deposit_balance,
        deposit_amount - command_price
    );
    assert_eq!(
        user_account_after.lamports,
        user_pda_lamports_before - command_price
    );

    // Assert admin balances
    assert_eq!(
        admin_profile_after.balance,
        admin_profile_before.balance + command_price
    );
    assert_eq!(
        admin_account_after.lamports,
        admin_pda_lamports_before + command_price
    );

    println!("✅ User Dispatch Command Test Passed!");
    println!(
        "   -> User balance changed: {} -> {}",
        deposit_amount, user_profile_after.deposit_balance
    );
    println!(
        "   -> Admin balance changed: {} -> {}",
        admin_profile_before.balance, admin_profile_after.balance
    );
}
