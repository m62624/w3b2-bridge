//! This module contains all integration tests for Admin-related instructions.
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
use w3b2_bridge_program::state::{AdminProfile, UserProfile};

/// Tests the successful creation of an `AdminProfile` PDA.
///
/// ### Scenario
/// A new service provider registers their profile on the protocol.
///
/// ### Arrange
/// 1. A new `Keypair` is created and funded to act as the admin's `ChainCard` (`authority`).
/// 2. A `Keypair` is created for the admin's off-chain communication key.
///
/// ### Act
/// The `admin::create_profile` helper is called to initialize the on-chain `AdminProfile` PDA.
///
/// ### Assert
/// 1. The `authority` and `communication_pubkey` fields in the new `AdminProfile` are set correctly.
/// 2. The `prices` vector is empty and the `balance` is 0.
/// 3. The account's lamport balance is exactly the rent-exempt minimum for its initial allocated size.
#[test]
fn test_admin_create_profile_success() {
    // === 1. Arrange (Setup) ===
    let mut svm = setup_svm();

    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let comm_key = create_keypair();

    // === 2. Act (Execution) ===
    println!("Attempting to create admin profile...");
    let admin_pda = admin::create_profile(&mut svm, &authority, comm_key.pubkey());
    println!("Admin profile created successfully at: {}", admin_pda);

    // === 3. Assert (Verification) ===
    let admin_account_data = svm.get_account(&admin_pda).unwrap();
    let admin_profile =
        AdminProfile::try_deserialize(&mut admin_account_data.data.as_slice()).unwrap();

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

/// Tests the successful update of an `AdminProfile`'s communication key.
///
/// ### Scenario
/// An admin with an existing profile wants to change their key for off-chain communication.
///
/// ### Arrange
/// 1. An `AdminProfile` is created with an initial communication key.
/// 2. A new `Keypair` is generated for the new communication key.
///
/// ### Act
/// The `admin::update_comm_key` helper is called.
///
/// ### Assert
/// 1. The `communication_pubkey` field in the `AdminProfile` is updated to the new key.
/// 2. Other fields, like `authority`, remain unchanged.
#[test]
fn test_admin_update_comm_key_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();
    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    let initial_comm_key = create_keypair();
    let admin_pda = admin::create_profile(&mut svm, &authority, initial_comm_key.pubkey());

    let new_comm_key = create_keypair();

    // === 2. Act ===
    println!("Updating communication key...");
    admin::update_comm_key(&mut svm, &authority, new_comm_key.pubkey());

    // === 3. Assert ===
    let admin_account_data = svm.get_account(&admin_pda).unwrap();
    let admin_profile =
        AdminProfile::try_deserialize(&mut admin_account_data.data.as_slice()).unwrap();

    assert_eq!(admin_profile.communication_pubkey, new_comm_key.pubkey());
    assert_ne!(
        admin_profile.communication_pubkey,
        initial_comm_key.pubkey()
    );
    assert_eq!(admin_profile.authority, authority.pubkey());

    println!("✅ Update Comm Key Test Passed!");
    println!("   -> Old Key: {}", initial_comm_key.pubkey());
    println!("   -> New Key: {}", admin_profile.communication_pubkey);
}

/// Tests the successful closure of an `AdminProfile` account.
///
/// ### Scenario
/// An admin unregisters their service and recovers the rent lamports held by the PDA.
///
/// ### Arrange
/// 1. An `AdminProfile` is created.
/// 2. The lamport balances of the admin's `ChainCard` and the `AdminProfile` PDA are recorded.
///
/// ### Act
/// The `admin::close_profile` helper is called.
///
/// ### Assert
/// 1. The `AdminProfile` PDA account no longer exists.
/// 2. The balance of the admin's `ChainCard` (`authority`) has increased by the lamport
///    balance of the closed PDA, minus the transaction fee.
#[test]
fn test_admin_close_profile_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();
    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let comm_key = create_keypair();

    let admin_pda = admin::create_profile(&mut svm, &authority, comm_key.pubkey());

    let pda_balance = svm.get_balance(&admin_pda).unwrap();
    let authority_balance_before = svm.get_balance(&authority.pubkey()).unwrap();

    // === 2. Act ===
    println!("Closing admin profile...");
    admin::close_profile(&mut svm, &authority);
    println!("Profile closed.");

    // === 3. Assert ===
    let closed_account = svm.get_account(&admin_pda);
    assert!(closed_account.is_none(), "Account was not closed!");

    let authority_balance_after = svm.get_balance(&authority.pubkey()).unwrap();
    let expected_balance = authority_balance_before + pda_balance - 5000;
    assert_eq!(authority_balance_after, expected_balance);

    println!("✅ Close Profile Test Passed!");
    println!(
        "   -> Authority balance correctly refunded: {} -> {}",
        authority_balance_before, authority_balance_after
    );
}

/// Tests the successful update of an admin's price list and the `realloc` feature.
///
/// ### Scenario
/// An admin sets or changes the prices for their services, which requires resizing the PDA.
///
/// ### Arrange
/// 1. An `AdminProfile` is created with an empty price list.
/// 2. A new price list is defined.
///
/// ### Act
/// The `admin::update_prices` helper is called.
///
/// ### Assert
/// 1. The `prices` vector in the account data is updated correctly.
/// 2. The on-chain account size (`data.len()`) has changed to accommodate the new vector size.
#[test]
fn test_admin_update_prices_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();
    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let comm_key = create_keypair();

    let admin_pda = admin::create_profile(&mut svm, &authority, comm_key.pubkey());

    let new_prices = vec![(1, 1000), (2, 2500), (5, 10000)];

    let account_before = svm.get_account(&admin_pda).unwrap();
    let size_before = account_before.data.len();

    // === 2. Act ===
    println!("Updating prices for admin profile...");
    admin::update_prices(&mut svm, &authority, new_prices.clone());
    println!("Prices updated.");

    // === 3. Assert ===
    let account_after = svm.get_account(&admin_pda).unwrap();
    let size_after = account_after.data.len();
    let admin_profile = AdminProfile::try_deserialize(&mut account_after.data.as_slice()).unwrap();

    assert_eq!(admin_profile.prices, new_prices);

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

/// Tests the successful dispatch of a command *from* an admin *to* a user.
///
/// ### Scenario
/// An admin sends a notification or command to a user associated with their service.
/// This is a non-financial transaction.
///
/// ### Arrange
/// 1. An `AdminProfile` is created.
/// 2. A `UserProfile` is created and linked to that admin.
/// 3. The initial state of all accounts is recorded.
///
/// ### Act
/// The `admin::dispatch_command` helper is called by the admin.
///
/// ### Assert
/// 1. The internal balances (`balance`, `deposit_balance`) of both profiles are unchanged.
/// 2. The on-chain lamport balances of both PDAs are also unchanged.
/// 3. The admin's `authority` wallet balance decreases only by the transaction fee.
#[test]
fn test_admin_dispatch_command_success() {
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

    let admin_account_before = svm.get_account(&admin_pda).unwrap();
    let admin_profile_before =
        AdminProfile::try_deserialize(&mut admin_account_before.data.as_slice()).unwrap();

    let user_account_before = svm.get_account(&user_pda).unwrap();
    let user_profile_before =
        UserProfile::try_deserialize(&mut user_account_before.data.as_slice()).unwrap();

    let admin_authority_lamports_before = svm.get_balance(&admin_authority.pubkey()).unwrap();

    // === 2. Act ===
    println!("Admin dispatching command to user...");
    admin::dispatch_command(
        &mut svm,
        &admin_authority,
        user_pda,
        101, // Notification command ID
        vec![4, 5, 6],
    );
    println!("Command dispatched successfully.");

    // === 3. Assert ===
    let admin_account_after = svm.get_account(&admin_pda).unwrap();
    let admin_profile_after =
        AdminProfile::try_deserialize(&mut admin_account_after.data.as_slice()).unwrap();

    let user_account_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_after.data.as_slice()).unwrap();

    let admin_authority_lamports_after = svm.get_balance(&admin_authority.pubkey()).unwrap();

    // Assert that internal balances are unchanged
    assert_eq!(admin_profile_after.balance, admin_profile_before.balance);
    assert_eq!(
        user_profile_after.deposit_balance,
        user_profile_before.deposit_balance
    );

    // Assert that PDA lamport balances are unchanged
    assert_eq!(admin_account_after.lamports, admin_account_before.lamports);
    assert_eq!(user_account_after.lamports, user_account_before.lamports);

    // Assert admin's signer balance only changed by the transaction fee
    let expected_admin_authority_balance = admin_authority_lamports_before - 5000;
    assert_eq!(
        admin_authority_lamports_after,
        expected_admin_authority_balance
    );

    println!("✅ Admin Dispatch Command Test Passed!");
    println!(
        "   -> Balances remained unchanged (Admin: {}, User: {})",
        admin_profile_after.balance, user_profile_after.deposit_balance
    );
}

/// Tests the successful withdrawal of *earned* funds by an admin.
///
/// ### Scenario
/// This is an integration test. A user pays an admin for a service via `user_dispatch_command`,
/// and then the admin withdraws a portion of those earnings to an external wallet.
///
/// ### Arrange
/// 1. An admin and a user are created.
/// 2. The admin sets a price for a command.
/// 3. The user deposits funds.
/// 4. The user calls the paid command, transferring funds to the admin's balance.
/// 5. The admin's state is recorded before the withdrawal.
///
/// ### Act
/// The `admin::withdraw` helper is called.
///
/// ### Assert
/// 1. The admin's internal `balance` decreases by the withdrawal amount.
/// 2. The admin PDA's on-chain lamport balance also decreases by the same amount.
/// 3. The destination wallet's balance increases by the withdrawal amount.
#[test]
fn test_admin_withdraw_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    // Create Admin and set a price for a service
    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());
    let command_price = LAMPORTS_PER_SOL;
    admin::update_prices(&mut svm, &admin_authority, vec![(1, command_price)]);

    // Create a User who will pay the Admin
    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let _ = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    // User deposits funds into their profile
    let deposit_amount = 2 * LAMPORTS_PER_SOL;
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    // User "buys" the service, transferring funds to the Admin
    println!("User pays admin {} lamports...", command_price);
    user::dispatch_command(&mut svm, &user_authority, admin_pda, 1, vec![1, 2, 3]);

    // Prepare for the withdrawal
    let destination_wallet = create_keypair();
    let withdraw_amount = command_price / 2; // Withdraw half of the earnings

    // Get state *before* the withdrawal
    let pda_account_before = svm.get_account(&admin_pda).unwrap();
    let pda_lamports_before = pda_account_before.lamports;
    let admin_profile_before =
        AdminProfile::try_deserialize(&mut pda_account_before.data.as_slice()).unwrap();
    let destination_balance_before = 0;

    assert_eq!(admin_profile_before.balance, command_price);

    // === 2. Act ===
    println!("Admin withdrawing {} lamports...", withdraw_amount);
    admin::withdraw(
        &mut svm,
        &admin_authority,
        destination_wallet.pubkey(),
        withdraw_amount,
    );
    println!("Withdrawal successful.");

    // === 3. Assert ===
    let pda_account_after = svm.get_account(&admin_pda).unwrap();
    let admin_profile_after =
        AdminProfile::try_deserialize(&mut pda_account_after.data.as_slice()).unwrap();
    let destination_balance_after = svm.get_balance(&destination_wallet.pubkey()).unwrap();

    assert_eq!(
        admin_profile_after.balance,
        admin_profile_before.balance - withdraw_amount
    );
    assert_eq!(
        pda_account_after.lamports,
        pda_lamports_before - withdraw_amount
    );
    assert_eq!(
        destination_balance_after,
        destination_balance_before + withdraw_amount
    );

    println!("✅ Admin Withdraw Test Passed!");
    println!(
        "   -> PDA internal balance is now: {}",
        admin_profile_after.balance
    );
    println!(
        "   -> Destination wallet received: {} lamports",
        destination_balance_after
    );
}
