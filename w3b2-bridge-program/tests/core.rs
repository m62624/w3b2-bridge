#![allow(deprecated)]

use anchor_lang::InstructionData;
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use borsh::{BorshDeserialize, BorshSerialize};
use lazy_static::lazy_static;
use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_program::rent::Rent;
use solana_program::system_program;
use solana_sdk::{
    instruction::AccountMeta, instruction::Instruction, message::Message,
    message::VersionedMessage, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::VersionedTransaction,
};
use w3b2_bridge_program::state::FundingRequest;
use w3b2_bridge_program::protocol::FundingStatus;

const PATH_SBF: &str = "../target/deploy/w3b2_bridge_program.so";

lazy_static! {
    static ref PROGRAM_ID: Pubkey = Pubkey::new_from_array(w3b2_bridge_program::id().to_bytes());
}

fn anchor_discriminator(method: &str) -> [u8; 8] {
    let mut h = Sha256::new();
    h.update(format!("global:{}", method).as_bytes());
    let out = h.finalize();
    let mut d = [0u8; 8];
    d.copy_from_slice(&out[..8]);
    d
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RegisterAdminArgs {
    pub funding_amount: u64,
}

fn register_admin_helper(
    svm: &mut LiteSVM,
    payer: &Keypair,
    admin_authority: &Keypair,
    co_signer: &Keypair,
    funding_amount: u64,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    let (_, bump) =
        Pubkey::find_program_address(&[b"admin", co_signer.pubkey().as_ref()], &*PROGRAM_ID);
    let admin_pda = Pubkey::create_program_address(
        &[b"admin", co_signer.pubkey().as_ref(), &[bump]],
        &*PROGRAM_ID,
    )?;

    let data = w3b2_bridge_program::instruction::RegisterAdmin { funding_amount }.data();
    let accounts = vec![
        AccountMeta::new(admin_pda, false),                        // PDA
        AccountMeta::new(payer.pubkey(), true),                    // payer
        AccountMeta::new_readonly(admin_authority.pubkey(), true), // authority
        AccountMeta::new_readonly(co_signer.pubkey(), true),       // co_signer
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    let ix = Instruction {
        program_id: *PROGRAM_ID,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[payer, admin_authority, co_signer],
    )?;

    svm.send_transaction(tx)
        .map_err(|e| anyhow::anyhow!("register_admin transaction failed: {:?}", e))?;

    Ok(admin_pda)
}

mod fn_register_admin {
    use super::*;

    #[test]
    fn test_register_admin_success() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let admin_authority = Keypair::new();
        let co_signer = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&admin_authority.pubkey(), 0).unwrap();
        svm.airdrop(&co_signer.pubkey(), 0).unwrap();

        let funding_amount = 10_000;
        let admin_pda = register_admin_helper(
            &mut svm,
            &payer,
            &admin_authority,
            &co_signer,
            funding_amount,
        )
        .unwrap();

        let acc = svm.get_account(&admin_pda).unwrap();
        assert_eq!(acc.data[8..40], admin_authority.pubkey().to_bytes());
        assert_eq!(acc.data[40..72], co_signer.pubkey().to_bytes());
        let rent = Rent::default();
        let min_balance = rent.minimum_balance(acc.data.len()) + funding_amount;
        assert!(acc.lamports >= min_balance, "PDA balance too low");
    }

    #[test]
    fn test_register_admin_insufficient_balance() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let admin_authority = Keypair::new();
        let co_signer = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        // payer имеет недостаточно lamports
        svm.airdrop(&payer.pubkey(), 1_000_000).unwrap();
        svm.airdrop(&admin_authority.pubkey(), 0).unwrap();
        svm.airdrop(&co_signer.pubkey(), 0).unwrap();

        let funding_amount = 1_000_000_000;
        let res = register_admin_helper(
            &mut svm,
            &payer,
            &admin_authority,
            &co_signer,
            funding_amount,
        );
        println!("Result: {:?}", res);
        assert!(
            res.is_err(),
            "Should fail due to insufficient payer balance"
        );
    }

    #[test]
    fn test_register_admin_already_registered() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let admin_authority = Keypair::new();
        let co_signer = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&admin_authority.pubkey(), 1_000_000_000)
            .unwrap();
        svm.airdrop(&co_signer.pubkey(), 0).unwrap();

        let funding_amount = 1_000_000;
        let admin_pda = register_admin_helper(
            &mut svm,
            &payer,
            &admin_authority,
            &co_signer,
            funding_amount,
        )
        .unwrap();
        println!("First registration PDA: {}", admin_pda);

        // Попытка повторной регистрации -> должна упасть
        let res = register_admin_helper(
            &mut svm,
            &payer,
            &admin_authority,
            &co_signer,
            funding_amount,
        );
        println!("Second registration result: {:?}", res);
        assert!(res.is_err(), "Second registration should fail");
    }

    #[test]
    fn test_create_several_admins_with_different_signers() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let admin1_authority = Keypair::new();
        let co_signer1 = Keypair::new();
        let admin2_authority = Keypair::new();
        let co_signer2 = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&admin1_authority.pubkey(), 1_000_000_000)
            .unwrap();
        svm.airdrop(&co_signer1.pubkey(), 0).unwrap();
        svm.airdrop(&admin2_authority.pubkey(), 1_000_000_000)
            .unwrap();
        svm.airdrop(&co_signer2.pubkey(), 0).unwrap();

        let funding_amount = 1_000_000;
        let admin1_pda = register_admin_helper(
            &mut svm,
            &payer,
            &admin1_authority,
            &co_signer1,
            funding_amount,
        )
        .unwrap();
        println!("Admin1 PDA: {}", admin1_pda);

        let admin2_pda = register_admin_helper(
            &mut svm,
            &payer,
            &admin2_authority,
            &co_signer2,
            funding_amount,
        )
        .unwrap();
        println!("Admin2 PDA: {}", admin2_pda);

        assert_ne!(admin1_pda, admin2_pda, "PDAs should be different");
    }
}

fn register_user_helper(
    svm: &mut LiteSVM,
    payer: &Keypair,
    user_wallet: &Keypair,
    co_signer: &Keypair,
    initial_balance: u64,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    let (_, bump) =
        Pubkey::find_program_address(&[b"user", co_signer.pubkey().as_ref()], &*PROGRAM_ID);
    let user_pda = Pubkey::create_program_address(
        &[b"user", co_signer.pubkey().as_ref(), &[bump]],
        &*PROGRAM_ID,
    )?;

    let data = w3b2_bridge_program::instruction::RegisterUser { initial_balance }.data();
    let accounts = vec![
        AccountMeta::new(user_pda, false),                     // PDA
        AccountMeta::new(payer.pubkey(), true),                // payer
        AccountMeta::new_readonly(user_wallet.pubkey(), true), // user_wallet
        AccountMeta::new_readonly(co_signer.pubkey(), true),   // co_signer
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    let ix = Instruction {
        program_id: *PROGRAM_ID,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[payer, user_wallet, co_signer],
    )?;

    svm.send_transaction(tx).map_err(|e| {
        anyhow::anyhow!(
            "register_user transaction failed: {:?}, user_pda: {}",
            e,
            user_pda
        )
    })?;
    Ok(user_pda)
}

mod fn_register_user {
    use super::*;
    use solana_program::rent::Rent;

    #[test]
    fn test_register_user_success() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let user_wallet = Keypair::new();
        let co_signer = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&user_wallet.pubkey(), 0).unwrap();
        svm.airdrop(&co_signer.pubkey(), 0).unwrap();

        let initial_balance = 10_000;
        let user_pda =
            register_user_helper(&mut svm, &payer, &user_wallet, &co_signer, initial_balance)
                .unwrap();

        let acc = svm.get_account(&user_pda).unwrap();
        assert_eq!(acc.data[8..40], user_wallet.pubkey().to_bytes());
        assert_eq!(acc.data[40..72], co_signer.pubkey().to_bytes());
        let rent = Rent::default();
        let min_balance = rent.minimum_balance(acc.data.len()) + initial_balance;
        assert!(acc.lamports >= min_balance, "PDA balance too low");
    }

    #[test]
    fn test_register_user_insufficient_balance() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let user_wallet = Keypair::new();
        let co_signer = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 1_000).unwrap(); // слишком мало для funding
        svm.airdrop(&user_wallet.pubkey(), 0).unwrap();
        svm.airdrop(&co_signer.pubkey(), 0).unwrap();

        let initial_balance = 1_000_000;
        let res = register_user_helper(&mut svm, &payer, &user_wallet, &co_signer, initial_balance);
        assert!(
            res.is_err(),
            "Should fail due to insufficient payer balance"
        );
    }

    #[test]
    fn test_register_user_already_registered() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let user_wallet = Keypair::new();
        let co_signer = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&user_wallet.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&co_signer.pubkey(), 0).unwrap();

        let initial_balance = 1_000_000;
        let user_pda =
            register_user_helper(&mut svm, &payer, &user_wallet, &co_signer, initial_balance)
                .unwrap();

        let res = register_user_helper(&mut svm, &payer, &user_wallet, &co_signer, initial_balance);
        assert!(res.is_err(), "Second registration should fail");
    }

    #[test]
    fn test_create_several_users_with_different_signers() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let user1_wallet = Keypair::new();
        let co_signer1 = Keypair::new();
        let user2_wallet = Keypair::new();
        let co_signer2 = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&user1_wallet.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&co_signer1.pubkey(), 0).unwrap();
        svm.airdrop(&user2_wallet.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&co_signer2.pubkey(), 0).unwrap();

        let initial_balance = 1_000_000;
        let user1_pda = register_user_helper(
            &mut svm,
            &payer,
            &user1_wallet,
            &co_signer1,
            initial_balance,
        )
        .unwrap();

        let user2_pda = register_user_helper(
            &mut svm,
            &payer,
            &user2_wallet,
            &co_signer2,
            initial_balance,
        )
        .unwrap();

        assert_ne!(user1_pda, user2_pda, "PDAs should be different");
    }
}

fn request_funding_only_helper(
    svm: &mut LiteSVM,
    payer: &Keypair,
    user_pda: Pubkey, // Теперь принимаем user_pda как Pubkey
    amount: u64,
    target_admin: Pubkey,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    // Correctly derive the funding_pda based on user_account PDA.
    let (_, bump) = Pubkey::find_program_address(
        &[
            b"funding",
            user_pda.as_ref(), // Use user_pda as a seed
            &payer.pubkey().to_bytes(),
        ],
        &*PROGRAM_ID,
    );
    let funding_pda = Pubkey::create_program_address(
        &[
            b"funding",
            user_pda.as_ref(), // Use user_pda as a seed
            &payer.pubkey().to_bytes(),
            &[bump],
        ],
        &*PROGRAM_ID,
    )?;

    let data = w3b2_bridge_program::instruction::RequestFunding {
        amount,
        target_admin,
    }
    .data();

    let accounts = vec![
        AccountMeta::new(funding_pda, false),
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new(user_pda, false), // Pass user_pda, not user_wallet
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let ix = Instruction {
        program_id: *PROGRAM_ID,
        accounts,
        data,
    };

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer])?; // Only payer is a signer

    svm.send_transaction(tx).map_err(|e| {
        anyhow::anyhow!(
            "request_funding_only transaction failed: {:?}, funding_pda: {}",
            e,
            funding_pda
        )
    })?;

    Ok(funding_pda)
}

mod funding {
    use super::*;
    #[test]
    fn test_request_funding_only() {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let admin_authority = Keypair::new();
        let admin_co = Keypair::new();
        let user_wallet = Keypair::new();
        let user_co = Keypair::new();

        svm.add_program_from_file(*PROGRAM_ID, PATH_SBF).unwrap();

        // Airdrop
        svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
        svm.airdrop(&admin_authority.pubkey(), 1_000_000_000)
            .unwrap();
        svm.airdrop(&admin_co.pubkey(), 0).unwrap();
        svm.airdrop(&user_wallet.pubkey(), 1_000_000_000).unwrap();
        svm.airdrop(&user_co.pubkey(), 0).unwrap();

        // Регистрируем админа и пользователя
        let admin_pda =
            register_admin_helper(&mut svm, &payer, &admin_authority, &admin_co, 1_000_000)
                .unwrap();
        let user_pda =
            register_user_helper(&mut svm, &payer, &user_wallet, &user_co, 100_000).unwrap();

        // Запрос funding без approval
        let request_amount = 50_000;
        let funding_pda = request_funding_only_helper(
            &mut svm,
            &payer,
            user_pda, // Передаем user_pda
            request_amount,
            admin_pda,
        )
        .unwrap();

        // Проверяем FundingRequest account
        let acc = svm.get_account(&funding_pda).unwrap();
        assert!(acc.lamports > 0, "Funding PDA should exist");

        // Баланс пользователя ещё не увеличен
        let user_acc = svm.get_account(&user_pda).unwrap();
        println!("User PDA balance: {}", user_acc.lamports);
        assert_eq!(
            user_acc.lamports,
            100_000 + Rent::default().minimum_balance(user_acc.data.len())
        );
    }
}
