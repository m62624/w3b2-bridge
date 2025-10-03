// use solana_client::nonblocking::rpc_client::RpcClient;
// use solana_sdk::commitment_config::CommitmentConfig;
// use solana_sdk::pubkey::Pubkey;
// use solana_sdk::signature::Keypair;
// use solana_sdk::signer::Signer;
// use std::sync::Arc;
// use std::{collections::HashMap, time::Duration};
// use w3b2_connector::{
//     client::OnChainClient,
//     keystore::{ChainCard, ChainCardFactory},
// };

// const URL_LOCALHOST: &str = "http://127.0.0.1:8899";

// pub fn setup_temp_db() -> sled::Db {
//     sled::Config::new().temporary(true).open().unwrap()
// }

// fn setup_client(rpc_client: Arc<RpcClient>) -> (OnChainClient, Arc<ChainCard>) {
//     let (chain_card, _mnemonic) = ChainCardFactory::generate_new(None, HashMap::new()).unwrap();
//     let chain_card_arc = Arc::new(chain_card);

//     // `OnChainClient::new` теперь получает то, что ожидает.
//     let client = OnChainClient::new(rpc_client, chain_card_arc.clone());

//     (client, chain_card_arc)
// }

// #[tokio::test]
// async fn test_admin_profile_creation() -> Result<(), anyhow::Error> {
//     let rpc_client = Arc::new(RpcClient::new_with_commitment(
//         URL_LOCALHOST.to_string(),
//         CommitmentConfig::confirmed(),
//     ));

//     let (client, admin_card) = setup_client(rpc_client.clone());

//     println!("Requesting airdrop for {}...", admin_card.authority());
//     let signature = rpc_client
//         .request_airdrop(&admin_card.authority(), 1_000_000_000) // 1 SOL
//         .await?;

//     println!(
//         "Waiting for airdrop confirmation for signature {}...",
//         signature
//     );
//     loop {
//         let statuses = rpc_client.get_signature_statuses(&[signature]).await?.value;
//         match &statuses[0] {
//             Some(status) if status.err.is_none() => {
//                 println!("Airdrop confirmed!");
//                 break;
//             }
//             Some(status) if status.err.is_some() => {
//                 panic!("Airdrop transaction failed with error: {:?}", status.err);
//             }
//             _ => {
//                 // Ждем немного перед повторной проверкой
//                 tokio::time::sleep(Duration::from_millis(500)).await;
//             }
//         }
//     }

//     let comm_key = Keypair::new();

//     // 5. Теперь создаем профиль админа. Эта транзакция больше не должна падать из-за нехватки средств.
//     println!("Creating admin profile...");
//     let result = client.admin_register_profile(comm_key.pubkey()).await;

//     assert!(
//         result.is_ok(),
//         "Failed to create admin profile: {:?}",
//         result.err()
//     );

//     let signature = result.unwrap();
//     println!(
//         "Admin profile created successfully! Signature: {}",
//         signature
//     );

//     // 6. Проверяем, что аккаунт действительно был создан на блокчейне.
//     let (admin_pda, _) = Pubkey::find_program_address(
//         &[b"admin", admin_card.authority().as_ref()],
//         &w3b2_bridge_program::ID,
//     );

//     println!("Verifying PDA account at address {}...", admin_pda);
//     let account = rpc_client.get_account(&admin_pda).await?;

//     // Проверяем, что аккаунт не пустой (т.е. существует и имеет данные/лампорты)
//     assert_ne!(
//         account.lamports, 0,
//         "Admin profile PDA was not created or is empty"
//     );

//     println!("Test passed successfully!");

//     Ok(())
// }
