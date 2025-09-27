use crate::keystore::ChainCard;
use anchor_lang::{InstructionData, ToAccountMetas};
use solana_client::client_error::ClientError;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::{instruction::Instruction, transaction::Transaction};
use w3b2_bridge_program::state::UpdatePricesArgs;
use w3b2_bridge_program::{accounts, instruction, state::PriceEntry};

use std::sync::Arc;

/// A lightweight, clonable client for interacting with the W3B2 Bridge Program.
///
/// This client is designed to be instantiated for a specific `ChainCard`, representing
/// a single user or admin identity. It shares a common `RpcClient` instance via an `Arc`
/// to efficiently manage connections to the Solana cluster.
#[derive(Clone)]
pub struct OnChainClient {
    /// A shared, thread-safe reference to the Solana JSON RPC client.
    rpc_client: Arc<RpcClient>,
    /// A shared, thread-safe reference to the `ChainCard` identity that this client
    /// will use to sign and pay for all transactions.
    chain_card: Arc<ChainCard>,
}

impl OnChainClient {
    /// Creates a new on-chain client session for a specific `ChainCard`.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - A shared `Arc<RpcClient>` for communicating with the Solana cluster.
    /// * `chain_card` - A shared `Arc<ChainCard>` representing the identity that will sign transactions.
    pub fn new(rpc_client: Arc<RpcClient>, chain_card: Arc<ChainCard>) -> Self {
        Self {
            rpc_client,
            chain_card,
        }
    }

    /// Returns a reference to the underlying `RpcClient`.
    pub fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }

    /// Returns a reference to the underlying `ChainCard` identity.
    pub fn chain_card(&self) -> &ChainCard {
        &self.chain_card
    }

    /// A private helper function to build, sign, and send a transaction
    /// containing a single instruction.
    ///
    /// This method handles fetching the latest blockhash, signing the transaction
    /// with the instance's `ChainCard`, and sending it to the cluster for confirmation.
    ///
    /// # Arguments
    ///
    /// * `ix` - The single `Instruction` to be included in the transaction.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Signature` of the confirmed transaction, or a `ClientError`.
    async fn send_tx(&self, ix: Instruction) -> Result<Signature, ClientError> {
        let mut tx = Transaction::new_with_payer(&[ix], Some(&self.chain_card.authority()));
        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
        tx.sign(&[self.chain_card.keypair()], recent_blockhash);
        let signature = self.rpc_client.send_and_confirm_transaction(&tx).await?;
        Ok(signature)
    }

    /// Sends an `admin_register_profile` transaction to initialize a new `AdminProfile` PDA.
    ///
    /// The new PDA will be owned by this client's `ChainCard` authority.
    ///
    /// # Arguments
    ///
    /// * `communication_pubkey` - The public key for secure off-chain communication.
    pub async fn admin_register_profile(
        &self,
        communication_pubkey: Pubkey,
    ) -> Result<Signature, ClientError> {
        let (admin_pda, _) = Pubkey::find_program_address(
            &[b"admin", self.chain_card.authority().as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminRegisterProfile {
                authority: self.chain_card.authority(),
                admin_profile: admin_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::AdminRegisterProfile {
                communication_pubkey,
            }
            .data(),
        };

        self.send_tx(ix).await
    }

    // NOTE: Place these methods inside the `impl OnChainClient` block from Part 1.

    // --- Admin Methods ---

    /// Sends an `admin_update_comm_key` transaction to update the communication public key.
    ///
    /// # Arguments
    ///
    /// * `new_key` - The new communication public key to set.
    pub async fn admin_update_comm_key(&self, new_key: Pubkey) -> Result<Signature, ClientError> {
        let (admin_pda, _) = Pubkey::find_program_address(
            &[b"admin", self.chain_card.authority().as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminUpdateCommKey {
                authority: self.chain_card.authority(),
                admin_profile: admin_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminUpdateCommKey { new_key }.data(),
        };

        self.send_tx(ix).await
    }

    /// Sends an `admin_update_prices` transaction to set a new service price list.
    ///
    /// This will trigger a `realloc` on the `AdminProfile` PDA to fit the new data.
    ///
    /// # Arguments
    ///
    /// * `new_prices` - A vector of `PriceEntry` structs defining the new prices.
    pub async fn admin_update_prices(
        &self,
        new_prices: Vec<PriceEntry>,
    ) -> Result<Signature, ClientError> {
        let (admin_pda, _) = Pubkey::find_program_address(
            &[b"admin", self.chain_card.authority().as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminUpdatePrices {
                authority: self.chain_card.authority(),
                admin_profile: admin_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::AdminUpdatePrices {
                args: UpdatePricesArgs { new_prices },
            }
            .data(),
        };

        self.send_tx(ix).await
    }

    /// Sends an `admin_withdraw` transaction to withdraw earned funds from the `AdminProfile`.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of lamports to withdraw.
    /// * `destination` - The public key of the account that will receive the funds.
    pub async fn admin_withdraw(
        &self,
        amount: u64,
        destination: Pubkey,
    ) -> Result<Signature, ClientError> {
        let (admin_pda, _) = Pubkey::find_program_address(
            &[b"admin", self.chain_card.authority().as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminWithdraw {
                authority: self.chain_card.authority(),
                admin_profile: admin_pda,
                destination,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::AdminWithdraw { amount }.data(),
        };

        self.send_tx(ix).await
    }

    /// Sends an `admin_close_profile` transaction to close the `AdminProfile` PDA.
    ///
    /// The rent lamports from the closed account will be refunded to the admin's authority `ChainCard`.
    pub async fn admin_close_profile(&self) -> Result<Signature, ClientError> {
        let (admin_pda, _) = Pubkey::find_program_address(
            &[b"admin", self.chain_card.authority().as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminCloseProfile {
                authority: self.chain_card.authority(),
                admin_profile: admin_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminCloseProfile {}.data(),
        };

        self.send_tx(ix).await
    }

    /// Sends an `admin_dispatch_command` transaction to send a command/notification to a user.
    ///
    /// # Arguments
    ///
    /// * `target_user_profile_pda` - The PDA address of the target `UserProfile`.
    /// * `command_id` - The identifier for the command being sent.
    /// * `payload` - A byte vector containing the command's payload.
    pub async fn admin_dispatch_command(
        &self,
        target_user_profile_pda: Pubkey,
        command_id: u64,
        payload: Vec<u8>,
    ) -> Result<Signature, ClientError> {
        let (admin_pda, _) = Pubkey::find_program_address(
            &[b"admin", self.chain_card.authority().as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminDispatchCommand {
                admin_authority: self.chain_card.authority(),
                admin_profile: admin_pda,
                user_profile: target_user_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminDispatchCommand {
                command_id,
                payload,
            }
            .data(),
        };

        self.send_tx(ix).await
    }

    // --- User Methods ---

    /// Sends a `user_create_profile` transaction to create a `UserProfile` PDA for a specific service.
    ///
    /// # Arguments
    ///
    /// * `target_admin_pda` - The PDA address of the `AdminProfile` this user is linking to.
    /// * `communication_pubkey` - The user's public key for off-chain communication.
    pub async fn user_create_profile(
        &self,
        target_admin_pda: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Result<Signature, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[
                b"user",
                self.chain_card.authority().as_ref(),
                target_admin_pda.as_ref(),
            ],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserCreateProfile {
                authority: self.chain_card.authority(),
                user_profile: user_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::UserCreateProfile {
                target_admin: target_admin_pda,
                communication_pubkey,
            }
            .data(),
        };

        self.send_tx(ix).await
    }

    /// Sends a `user_update_comm_key` transaction to update the user's communication key.
    ///
    /// # Arguments
    ///
    /// * `admin_profile_pda` - The PDA of the admin profile this user profile is linked to.
    /// * `new_key` - The new communication public key.
    pub async fn user_update_comm_key(
        &self,
        admin_profile_pda: Pubkey,
        new_key: Pubkey,
    ) -> Result<Signature, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[
                b"user",
                self.chain_card.authority().as_ref(),
                admin_profile_pda.as_ref(),
            ],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserUpdateCommKey {
                authority: self.chain_card.authority(),
                admin_profile: admin_profile_pda,
                user_profile: user_pda,
            }
            .to_account_metas(None),
            data: instruction::UserUpdateCommKey { new_key }.data(),
        };

        self.send_tx(ix).await
    }

    /// Sends a `user_deposit` transaction to add funds to the `UserProfile` deposit balance.
    ///
    /// # Arguments
    ///
    /// * `admin_profile_pda` - The PDA of the admin profile this user profile is linked to.
    /// * `amount` - The amount of lamports to deposit.
    pub async fn user_deposit(
        &self,
        admin_profile_pda: Pubkey,
        amount: u64,
    ) -> Result<Signature, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[
                b"user",
                self.chain_card.authority().as_ref(),
                admin_profile_pda.as_ref(),
            ],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserDeposit {
                authority: self.chain_card.authority(),
                admin_profile: admin_profile_pda,
                user_profile: user_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::UserDeposit { amount }.data(),
        };

        self.send_tx(ix).await
    }

    /// Sends a `user_withdraw` transaction to retrieve funds from the `UserProfile` deposit balance.
    ///
    /// # Arguments
    ///
    /// * `admin_profile_pda` - The PDA of the admin profile this user profile is linked to.
    /// * `amount` - The amount of lamports to withdraw.
    /// * `destination` - The public key of the account that will receive the funds.
    pub async fn user_withdraw(
        &self,
        admin_profile_pda: Pubkey,
        amount: u64,
        destination: Pubkey,
    ) -> Result<Signature, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[
                b"user",
                self.chain_card.authority().as_ref(),
                admin_profile_pda.as_ref(),
            ],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserWithdraw {
                authority: self.chain_card.authority(),
                admin_profile: admin_profile_pda,
                user_profile: user_pda,
                destination,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::UserWithdraw { amount }.data(),
        };

        self.send_tx(ix).await
    }

    /// Sends a `user_close_profile` transaction to close the `UserProfile` PDA.
    ///
    /// All remaining funds (deposit and rent) will be refunded to the user's `ChainCard`.
    ///
    /// # Arguments
    ///
    /// * `admin_profile_pda` - The PDA of the admin profile this user profile is linked to.
    pub async fn user_close_profile(
        &self,
        admin_profile_pda: Pubkey,
    ) -> Result<Signature, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[
                b"user",
                self.chain_card.authority().as_ref(),
                admin_profile_pda.as_ref(),
            ],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserCloseProfile {
                authority: self.chain_card.authority(),
                admin_profile: admin_profile_pda,
                user_profile: user_pda,
            }
            .to_account_metas(None),
            data: instruction::UserCloseProfile {}.data(),
        };

        self.send_tx(ix).await
    }

    // --- Operational Methods ---

    /// Sends a `user_dispatch_command` transaction to call a service's API.
    ///
    /// This is the primary operational instruction for users. It will handle payment
    /// if the command has a non-zero price in the admin's price list.
    ///
    /// # Arguments
    ///
    /// * `admin_profile_pda` - The PDA of the target `AdminProfile` service.
    /// * `command_id` - The identifier of the command to execute.
    /// * `payload` - A byte vector containing the command's payload.
    pub async fn user_dispatch_command(
        &self,
        admin_profile_pda: Pubkey,
        command_id: u16,
        payload: Vec<u8>,
    ) -> Result<Signature, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[
                b"user",
                self.chain_card.authority().as_ref(),
                admin_profile_pda.as_ref(),
            ],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserDispatchCommand {
                authority: self.chain_card.authority(),
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::UserDispatchCommand {
                command_id,
                payload,
            }
            .data(),
        };

        self.send_tx(ix).await
    }

    /// Sends a `log_action` transaction to record an off-chain event on the blockchain.
    ///
    /// # Arguments
    ///
    /// * `session_id` - A generic identifier for grouping related actions.
    /// * `action_code` - A numeric code representing the specific action that occurred.
    pub async fn log_action(
        &self,
        session_id: u64,
        action_code: u16,
    ) -> Result<Signature, ClientError> {
        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::LogAction {
                authority: self.chain_card.authority(),
            }
            .to_account_metas(None),
            data: instruction::LogAction {
                session_id,
                action_code,
            }
            .data(),
        };

        self.send_tx(ix).await
    }
}

// Custom Debug implementation to avoid printing the entire RpcClient.
impl std::fmt::Debug for OnChainClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnChainClient")
            .field("rpc_client", &"&RpcClient")
            .field("chain_card", &self.chain_card)
            .finish()
    }
}
