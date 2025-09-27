// w3b2-connector/src/keystore.rs
//! Keystore implementation for w3b2-connector
//!
//! - Provides password-based encryption of BIP-39 mnemonic phrases using Argon2 + AES-256-GCM.
//! - Persists encrypted records into a sled DB as JSON blobs (StorableCard).
//! - Returns `ChainCard` (unlocked) when loading a card with the correct password.
//!
//! Security design summary:
//! - Argon2 derives a 32-byte key from a user password and a per-card random salt (`SaltString`).
//! - AES-256-GCM encrypts the mnemonic with a randomly generated 96-bit nonce per encryption.
//! - Nonce + salt + ciphertext are stored in the DB. The mnemonic is not stored in plaintext.
//! - Sensitive data (derived key) is wrapped in `zeroize::Zeroizing` so it is zeroed on drop.
//!
//! Notes:
//! - Keep Argon2 parameters tuned for your threat model (work factor vs ux).
//! - This module tries to minimize in-memory lifetime of secrets, but the OS may swap; consider secure environments.

use aes_gcm::{
    aead::{generic_array::GenericArray, Aead, Key},
    AeadCore, Aes256Gcm, KeyInit,
};
use anyhow::{anyhow, Result};
use argon2::password_hash::{rand_core::OsRng as RandCoreOsRng, SaltString};
use argon2::Argon2;
use bip39::Mnemonic;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signature::Signer, signer::keypair::keypair_from_seed,
};
use std::collections::HashMap;
use std::str::FromStr;
use zeroize::Zeroizing;

/// Expected nonce length for AES-GCM (96 bits = 12 bytes).
const AES_GCM_NONCE_LEN: usize = 12;

/// Public crypto utility helpers.
///
/// - `derive_key` uses Argon2 (Argon2id via `Argon2::default()`).
/// - `encrypt` returns `(ciphertext, nonce)` where nonce is a 12-byte vector.
/// - `decrypt` accepts stored nonce bytes and returns the plaintext.
pub struct Crypto;

impl Crypto {
    /// Derive a 32-byte key from a password and a salt using Argon2.
    ///
    /// Returns a `Zeroizing<[u8; 32]>` which will be zeroed when dropped.
    pub fn derive_key(password: &SecretString, salt: &SaltString) -> Result<Zeroizing<[u8; 32]>> {
        let mut key = Zeroizing::new([0u8; 32]);
        Argon2::default()
            .hash_password_into(
                password.expose_secret().as_bytes(),
                salt.as_str().as_bytes(),
                &mut *key,
            )
            .map_err(|e| anyhow!("Argon2 key derivation failed: {}", e))?;
        Ok(key)
    }

    /// Encrypt plaintext with AES-256-GCM using the provided 32-byte key.
    ///
    /// Returns `(ciphertext, nonce_bytes)`. Nonce is 12 bytes (recommended for AES-GCM).
    pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>)> {
        let key_ga = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key_ga);

        // Generate a random 96-bit (12-byte) nonce.
        let mut rng = RandCoreOsRng;
        let nonce_ga = Aes256Gcm::generate_nonce(&mut rng);
        let nonce_bytes = nonce_ga.as_slice().to_vec();

        let ciphertext = cipher
            .encrypt(&nonce_ga, plaintext)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        Ok((ciphertext, nonce_bytes))
    }

    /// Decrypt ciphertext with AES-256-GCM using provided nonce and key.
    pub fn decrypt(ciphertext: &[u8], nonce: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
        if nonce.len() != AES_GCM_NONCE_LEN {
            return Err(anyhow!(
                "Invalid nonce length: got {}, expected {}",
                nonce.len(),
                AES_GCM_NONCE_LEN
            ));
        }

        let key_ga = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key_ga);

        let nonce_ga = GenericArray::from_slice(nonce);
        let plaintext = cipher.decrypt(nonce_ga, ciphertext).map_err(|e| {
            anyhow!(
                "Decryption failed (invalid password or corrupted data): {}",
                e
            )
        })?;

        Ok(plaintext)
    }
}

/// Represents an unlocked ChainCard — contains a `Keypair` and associated metadata.
#[derive(Debug)]
pub struct ChainCard {
    pub pubkey: Pubkey,
    keypair: Keypair,
    pub metadata: HashMap<String, String>,
}

impl ChainCard {
    /// Borrow the underlying `Keypair`.
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }
}

/// Stored representation persisted to sled.
#[derive(Debug, Serialize, Deserialize)]
struct StorableCard {
    encrypted_mnemonic_phrase: Vec<u8>,
    salt: String,
    nonce: Vec<u8>,
    metadata: HashMap<String, String>,
}

/// Metadata update operations.
#[derive(Debug)]
pub enum MetadataUpdate {
    Replace(HashMap<String, String>),
    Set(String, String),
    Delete(String),
}

/// Keystore trait.
#[async_trait::async_trait]
pub trait Keystore: Send + Sync {
    async fn create_new_card(
        &self,
        id: &str,
        password: SecretString,
        bip39_passphrase: Option<SecretString>,
        metadata: HashMap<String, String>,
    ) -> Result<(ChainCard, SecretString)>;

    async fn load_card(
        &self,
        id: &str,
        password: SecretString,
        bip39_passphrase: Option<SecretString>,
    ) -> Result<ChainCard>;

    async fn list_cards(&self) -> Result<HashMap<String, HashMap<String, String>>>;
    async fn update_metadata(&self, id: &str, update: MetadataUpdate) -> Result<()>;
    async fn delete_card(&self, id: &str) -> Result<()>;
}

/// Sled-backed keystore.
#[derive(Clone)]
pub struct SledKeystore {
    db: sled::Db,
}

impl SledKeystore {
    pub fn new(db: sled::Db) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl Keystore for SledKeystore {
    /// Create a new card identified by `id`.
    async fn create_new_card(
        &self,
        id: &str,
        password: SecretString,
        bip39_passphrase: Option<SecretString>,
        metadata: HashMap<String, String>,
    ) -> Result<(ChainCard, SecretString)> {
        if self.db.contains_key(id.as_bytes())? {
            return Err(anyhow!("Card with id '{}' already exists", id));
        }

        // Generate mnemonic and derive seed.
        let mnemonic = Mnemonic::generate(12)?;
        let mnemonic_phrase = SecretString::new(mnemonic.to_string().into_boxed_str());

        // Derive seed for keypair. bip39::Mnemonic::to_seed accepts &str passphrase; empty string if None.
        let bip39_pass = bip39_passphrase.as_ref().map_or("", |p| p.expose_secret());
        let seed = mnemonic.to_seed(bip39_pass);

        // IMPORTANT: use solana helper to construct Keypair from seed.
        // This follows solana-keygen behaviour: it uses the first 32 bytes of the seed.
        let keypair = keypair_from_seed(seed.as_ref())
            .map_err(|e| anyhow!("Failed to derive keypair from seed: {}", e))?;

        // Argon2 salt and key derivation.
        let salt = SaltString::generate(&mut RandCoreOsRng);
        let key_zero = Crypto::derive_key(&password, &salt)?;

        // Encrypt mnemonic using derived key. nonce and ciphertext are returned.
        let (encrypted_mnemonic_phrase, nonce_bytes) =
            Crypto::encrypt(mnemonic_phrase.expose_secret().as_bytes(), &*key_zero)
                .map_err(|e| anyhow!("Encryption stage failed: {}", e))?;

        let storable_data = StorableCard {
            encrypted_mnemonic_phrase,
            salt: salt.to_string(),
            nonce: nonce_bytes,
            metadata: metadata.clone(),
        };

        self.db
            .insert(id.as_bytes(), serde_json::to_vec(&storable_data)?)?;
        self.db.flush_async().await?;

        let card = ChainCard {
            pubkey: keypair.pubkey(),
            keypair,
            metadata,
        };
        Ok((card, mnemonic_phrase))
    }

    /// Load a stored card and decrypt using the provided `password`.
    async fn load_card(
        &self,
        id: &str,
        password: SecretString,
        bip39_passphrase: Option<SecretString>,
    ) -> Result<ChainCard> {
        let raw_data = self
            .db
            .get(id.as_bytes())?
            .ok_or_else(|| anyhow!("Card with id '{}' not found", id))?;
        let storable_data: StorableCard = serde_json::from_slice(&raw_data)
            .map_err(|e| anyhow!("Stored data is invalid: {}", e))?;

        let salt = SaltString::from_b64(&storable_data.salt)
            .map_err(|e| anyhow!("Invalid salt format: {}", e))?;
        let key_zero = Crypto::derive_key(&password, &salt)?;

        let decrypted_bytes = Crypto::decrypt(
            &storable_data.encrypted_mnemonic_phrase,
            &storable_data.nonce,
            &*key_zero,
        )?;

        let mnemonic_phrase = String::from_utf8(decrypted_bytes)
            .map_err(|e| anyhow!("Decrypted mnemonic is not UTF-8: {}", e))?;
        let mnemonic = Mnemonic::from_str(&mnemonic_phrase)?;

        let bip39_pass = bip39_passphrase.as_ref().map_or("", |p| p.expose_secret());
        let seed = mnemonic.to_seed(bip39_pass);

        let keypair = keypair_from_seed(seed.as_ref())
            .map_err(|e| anyhow!("Failed to derive keypair from seed: {}", e))?;

        let card = ChainCard {
            pubkey: keypair.pubkey(),
            keypair,
            metadata: storable_data.metadata,
        };

        Ok(card)
    }

    async fn list_cards(&self) -> Result<HashMap<String, HashMap<String, String>>> {
        let mut result = HashMap::new();
        for item in self.db.iter() {
            let (key_bytes, val_bytes) = item?;
            let id = String::from_utf8(key_bytes.to_vec())?;
            let storable_data: StorableCard = serde_json::from_slice(&val_bytes)?;
            result.insert(id, storable_data.metadata);
        }
        Ok(result)
    }

    async fn update_metadata(&self, id: &str, update: MetadataUpdate) -> Result<()> {
        let raw_data = self
            .db
            .get(id.as_bytes())?
            .ok_or_else(|| anyhow!("Card with id '{}' not found to update", id))?;
        let mut storable_data: StorableCard = serde_json::from_slice(&raw_data)?;

        match update {
            MetadataUpdate::Replace(new_map) => {
                storable_data.metadata = new_map;
            }
            MetadataUpdate::Set(key, value) => {
                storable_data.metadata.insert(key, value);
            }
            MetadataUpdate::Delete(key) => {
                storable_data.metadata.remove(&key);
            }
        }

        self.db
            .insert(id.as_bytes(), serde_json::to_vec(&storable_data)?)?;
        self.db.flush_async().await?;
        Ok(())
    }

    async fn delete_card(&self, id: &str) -> Result<()> {
        let removed = self.db.remove(id.as_bytes())?;
        if removed.is_none() {
            return Err(anyhow!("Card with id '{}' not found", id));
        }
        self.db.flush_async().await?;
        Ok(())
    }
}
