mod instructions;

use anchor_lang::AccountDeserialize;
use instructions::*;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program::sysvar::rent::Rent;
use solana_sdk::signature::Signer;
use w3b2_bridge_program::state::{AdminProfile, UserProfile};

#[test]
fn test_admin_create_profile_success() {
    // === 1. Arrange (Setup) ===

    // Initialize the Solana virtual machine and load our program.
    let mut svm = setup_svm();

    // Create a new keypair and fund it with 10 SOL. This keypair will act
    // as the `authority` for the new admin profile and pay for the transaction.
    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Create a separate keypair to simulate the off-chain communication key.
    let comm_key = create_keypair();

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
    let mut svm = setup_svm();
    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);

    // Create the profile with an initial key.
    let initial_comm_key = create_keypair();
    let admin_pda = admin::create_profile(&mut svm, &authority, initial_comm_key.pubkey());

    // Define the new key we want to update to.
    let new_comm_key = create_keypair();

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
    let mut svm = setup_svm();
    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let comm_key = create_keypair();

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
    let mut svm = setup_svm();
    let authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let comm_key = create_keypair();

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

#[test]
fn test_admin_dispatch_command_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    // -- Создаем Админа --
    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());

    // -- Создаем Пользователя, которому админ отправит команду --
    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let user_pda = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    // -- Сохраняем состояние всех счетов *перед* вызовом команды --
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
        101, // ID команды-уведомления
        vec![4, 5, 6],
    );
    println!("Command dispatched successfully.");

    // === 3. Assert ===
    // -- Получаем финальное состояние --
    let admin_account_after = svm.get_account(&admin_pda).unwrap();
    let admin_profile_after =
        AdminProfile::try_deserialize(&mut admin_account_after.data.as_slice()).unwrap();

    let user_account_after = svm.get_account(&user_pda).unwrap();
    let user_profile_after =
        UserProfile::try_deserialize(&mut user_account_after.data.as_slice()).unwrap();

    let admin_authority_lamports_after = svm.get_balance(&admin_authority.pubkey()).unwrap();

    // Assertion 1: Внутренние балансы не изменились
    assert_eq!(admin_profile_after.balance, admin_profile_before.balance);
    assert_eq!(
        user_profile_after.deposit_balance,
        user_profile_before.deposit_balance
    );

    // Assertion 2: Балансы лампортов на PDA-аккаунтах не изменились
    assert_eq!(admin_account_after.lamports, admin_account_before.lamports);
    assert_eq!(user_account_after.lamports, user_account_before.lamports);

    // Assertion 3: Баланс админа-подписанта уменьшился только на комиссию за транзакцию
    let expected_admin_authority_balance = admin_authority_lamports_before - 5000; // 5000 lamports for tx fee
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

#[test]
fn test_admin_withdraw_success() {
    // === 1. Arrange ===
    let mut svm = setup_svm();

    // -- Создаем Админа и устанавливаем цену на услугу --
    let admin_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let admin_pda = admin::create_profile(&mut svm, &admin_authority, create_keypair().pubkey());
    let command_price = 1 * LAMPORTS_PER_SOL;
    admin::update_prices(&mut svm, &admin_authority, vec![(1, command_price)]);

    // -- Создаем Пользователя, который заплатит Админу --
    let user_authority = create_funded_keypair(&mut svm, 10 * LAMPORTS_PER_SOL);
    let _ = user::create_profile(
        &mut svm,
        &user_authority,
        create_keypair().pubkey(),
        admin_pda,
    );

    // -- Пользователь вносит депозит на свой профиль --
    let deposit_amount = 2 * LAMPORTS_PER_SOL;
    user::deposit(&mut svm, &user_authority, admin_pda, deposit_amount);

    // -- Пользователь "покупает" услугу, деньги переходят к Админу --
    println!("User pays admin {} lamports...", command_price);
    user::dispatch_command(&mut svm, &user_authority, admin_pda, 1, vec![1, 2, 3]);

    // -- Готовимся к выводу средств --
    let destination_wallet = create_keypair();
    let withdraw_amount = command_price / 2; // Выводим половину заработанного

    // -- Сохраняем состояние *перед* выводом --
    let pda_account_before = svm.get_account(&admin_pda).unwrap();
    let pda_lamports_before = pda_account_before.lamports;
    let admin_profile_before =
        AdminProfile::try_deserialize(&mut pda_account_before.data.as_slice()).unwrap();
    let destination_balance_before = 0; // Новый кошелек

    // Убедимся, что у админа действительно есть деньги на внутреннем балансе
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

    // Assertion 1: Внутренний баланс админа в данных PDA уменьшился.
    assert_eq!(
        admin_profile_after.balance,
        admin_profile_before.balance - withdraw_amount
    );

    // Assertion 2: Баланс лампортов самого PDA уменьшился.
    assert_eq!(
        pda_account_after.lamports,
        pda_lamports_before - withdraw_amount
    );

    // Assertion 3: Баланс кошелька получателя увеличился.
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
