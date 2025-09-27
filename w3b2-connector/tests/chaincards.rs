// use argon2::password_hash::{rand_core::OsRng as RandCoreOsRng, SaltString};
// use bip39::Mnemonic;
// use secrecy::{ExposeSecret, SecretString};
// use std::collections::HashMap;
// use std::str::FromStr;
// use w3b2_connector::keystore::*;

// // Helper to create a temporary Sled DB for isolated tests.
// fn setup_temp_db() -> sled::Db {
//     sled::Config::new().temporary(true).open().unwrap()
// }

// #[test]
// fn test_crypto_derive_key() {
//     let password = SecretString::new("password123".to_string().into_boxed_str());
//     let salt = SaltString::generate(&mut RandCoreOsRng);

//     let key1 = Crypto::derive_key(&password, &salt).unwrap();
//     assert_eq!(key1.len(), 32);

//     let key2 = Crypto::derive_key(&password, &salt).unwrap();
//     assert_eq!(*key1, *key2);

//     let wrong_password = SecretString::new("wrongpassword".to_string().into_boxed_str());
//     let key3 = Crypto::derive_key(&wrong_password, &salt).unwrap();
//     assert_ne!(*key1, *key3);
// }

// #[test]
// fn test_crypto_encrypt_decrypt_roundtrip() {
//     let password = SecretString::new("password123".to_string().into_boxed_str());
//     let salt = SaltString::generate(&mut RandCoreOsRng);
//     let key = Crypto::derive_key(&password, &salt).unwrap();

//     let plaintext = b"hello world";
//     let (ciphertext, nonce) = Crypto::encrypt(plaintext, &*key).unwrap();

//     assert_ne!(plaintext.to_vec(), ciphertext);

//     let decrypted = Crypto::decrypt(&ciphertext, &nonce, &*key).unwrap();
//     assert_eq!(plaintext.to_vec(), decrypted);
// }

// #[test]
// fn test_crypto_decrypt_with_wrong_key() {
//     let password = SecretString::new("password123".to_string().into_boxed_str());
//     let salt = SaltString::generate(&mut RandCoreOsRng);
//     let key = Crypto::derive_key(&password, &salt).unwrap();

//     let wrong_password = SecretString::new("wrongpassword".to_string().into_boxed_str());
//     let wrong_key = Crypto::derive_key(&wrong_password, &salt).unwrap();

//     let plaintext = b"hello world";
//     let (ciphertext, nonce) = Crypto::encrypt(plaintext, &*key).unwrap();

//     let result = Crypto::decrypt(&ciphertext, &nonce, &*wrong_key);
//     assert!(result.is_err());
//     assert!(result
//         .unwrap_err()
//         .to_string()
//         .contains("Decryption failed"));
// }

// #[tokio::test]
// async fn test_create_and_load_card_no_passphrase() {
//     let db = setup_temp_db();
//     let keystore = SledKeystore::new(db);
//     let id = "test_card_1";
//     let password = SecretString::new("my-secret-password".to_string().into_boxed_str());
//     let metadata = HashMap::from([("name".to_string(), "My Test Card".to_string())]);

//     let (created_card, mnemonic) = keystore
//         .create_new_card(id, password.clone(), None, metadata.clone())
//         .await
//         .unwrap();

//     assert_eq!(created_card.metadata, metadata);
//     assert!(Mnemonic::from_str(mnemonic.expose_secret()).is_ok());

//     let loaded_card = keystore.load_card(id, password, None).await.unwrap();

//     assert_eq!(loaded_card.pubkey, created_card.pubkey);
//     assert_eq!(loaded_card.metadata, created_card.metadata);
// }

// #[tokio::test]
// async fn test_create_and_load_card_with_passphrase() {
//     let db = setup_temp_db();
//     let keystore = SledKeystore::new(db);
//     let id = "test_card_bip39";
//     let password = SecretString::new("my-secret-password".to_string().into_boxed_str());
//     let bip39_pass = Some(SecretString::new(
//         "bip39-passphrase".to_string().into_boxed_str(),
//     ));

//     let (created_card, _) = keystore
//         .create_new_card(id, password.clone(), bip39_pass.clone(), HashMap::new())
//         .await
//         .unwrap();

//     let loaded_card = keystore
//         .load_card(id, password.clone(), bip39_pass)
//         .await
//         .unwrap();
//     assert_eq!(loaded_card.pubkey, created_card.pubkey);

//     let wrong_bip39_pass = Some(SecretString::new(
//         "wrong-bip39-pass".to_string().into_boxed_str(),
//     ));
//     let loaded_card_wrong_bip39 = keystore
//         .load_card(id, password.clone(), wrong_bip39_pass)
//         .await
//         .unwrap();
//     assert_ne!(loaded_card_wrong_bip39.pubkey, created_card.pubkey);

//     let loaded_card_no_bip39 = keystore.load_card(id, password, None).await.unwrap();
//     assert_ne!(loaded_card_no_bip39.pubkey, created_card.pubkey);
// }

// #[tokio::test]
// async fn test_load_card_wrong_password() {
//     let db = setup_temp_db();
//     let keystore = SledKeystore::new(db);
//     let id = "test_card_2";
//     let password = SecretString::new("correct-password".to_string().into_boxed_str());
//     let wrong_password = SecretString::new("wrong-password".to_string().into_boxed_str());

//     keystore
//         .create_new_card(id, password, None, HashMap::new())
//         .await
//         .unwrap();

//     let result = keystore.load_card(id, wrong_password, None).await;
//     assert!(result.is_err());
// }

// #[tokio::test]
// async fn test_create_duplicate_card_fails() {
//     let db = setup_temp_db();
//     let keystore = SledKeystore::new(db);
//     let id = "duplicate_card";
//     let password = SecretString::new("password".to_string().into_boxed_str());

//     keystore
//         .create_new_card(id, password.clone(), None, HashMap::new())
//         .await
//         .unwrap();

//     let result = keystore
//         .create_new_card(id, password, None, HashMap::new())
//         .await;
//     assert!(result.is_err());
//     assert!(result.unwrap_err().to_string().contains("already exists"));
// }

// #[tokio::test]
// async fn test_list_and_update_metadata() {
//     let db = setup_temp_db();
//     let keystore = SledKeystore::new(db);
//     let id1 = "card1";
//     let id2 = "card2";
//     let password = SecretString::new("pw".to_string().into_boxed_str());

//     keystore
//         .create_new_card(
//             id1,
//             password.clone(),
//             None,
//             HashMap::from([("role".to_string(), "admin".to_string())]),
//         )
//         .await
//         .unwrap();
//     keystore
//         .create_new_card(
//             id2,
//             password.clone(),
//             None,
//             HashMap::from([("role".to_string(), "user".to_string())]),
//         )
//         .await
//         .unwrap();

//     let cards = keystore.list_cards().await.unwrap();
//     assert_eq!(cards.len(), 2);
//     assert_eq!(cards.get(id1).unwrap().get("role").unwrap(), "admin");
//     assert_eq!(cards.get(id2).unwrap().get("role").unwrap(), "user");

//     keystore
//         .update_metadata(
//             id1,
//             MetadataUpdate::Set("name".to_string(), "Admin Card".to_string()),
//         )
//         .await
//         .unwrap();
//     let card1_loaded = keystore
//         .load_card(
//             id1,
//             SecretString::new("pw".to_string().into_boxed_str()),
//             None,
//         )
//         .await
//         .unwrap();
//     assert_eq!(card1_loaded.metadata.get("name").unwrap(), "Admin Card");

//     keystore
//         .update_metadata(id1, MetadataUpdate::Delete("role".to_string()))
//         .await
//         .unwrap();
//     let card1_loaded = keystore
//         .load_card(
//             id1,
//             SecretString::new("pw".to_string().into_boxed_str()),
//             None,
//         )
//         .await
//         .unwrap();
//     assert!(!card1_loaded.metadata.contains_key("role"));

//     let new_meta = HashMap::from([("status".to_string(), "active".to_string())]);
//     keystore
//         .update_metadata(id2, MetadataUpdate::Replace(new_meta.clone()))
//         .await
//         .unwrap();
//     let card2_loaded = keystore
//         .load_card(
//             id2,
//             SecretString::new("pw".to_string().into_boxed_str()),
//             None,
//         )
//         .await
//         .unwrap();
//     assert_eq!(card2_loaded.metadata, new_meta);
// }

// #[tokio::test]
// async fn test_delete_card() {
//     let db = setup_temp_db();
//     let keystore = SledKeystore::new(db);
//     let id = "to_delete";
//     let password = SecretString::new("pw".to_string().into_boxed_str());

//     keystore
//         .create_new_card(id, password.clone(), None, HashMap::new())
//         .await
//         .unwrap();

//     keystore.delete_card(id).await.unwrap();

//     let result = keystore.load_card(id, password, None).await;
//     assert!(result.is_err());
// }

// #[tokio::test]
// async fn test_load_card_with_corrupted_data() {
//     let db = setup_temp_db();
//     let keystore = SledKeystore::new(db.clone());
//     let id = "corrupted";
//     let password = SecretString::new("pw".to_string().into_boxed_str());

//     keystore
//         .create_new_card(id, password.clone(), None, HashMap::new())
//         .await
//         .unwrap();

//     db.insert(id, b"not a valid json").unwrap();
//     db.flush().unwrap();

//     let result = keystore.load_card(id, password, None).await;
//     assert!(result.is_err());
// }
