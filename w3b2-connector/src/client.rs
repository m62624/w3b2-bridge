// File: w3b2-connector/src/client.rs

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_client::client_error::ClientError;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::Transaction;
use std::sync::Arc;
use w3b2_bridge_program::{
    accounts, instruction,
    state::{PriceEntry, UpdatePricesArgs},
};

/// A client for preparing on-chain transactions for remote signing.
///
/// This struct provides methods to construct unsigned transactions for every
/// instruction in the W3B2 Bridge Program. It is designed for a non-custodial
/// architecture where the private key never leaves the client's device.
/// The server-side component (like a gRPC gateway) uses this builder to create
/// a transaction, sends it to the client for signing, and then receives the
/// signed transaction back for submission.
#[derive(Clone)]
pub struct TransactionBuilder {
    /// A shared, thread-safe reference to the Solana JSON RPC client.
    rpc_client: Arc<RpcClient>,
}

impl TransactionBuilder {
    /// Creates a new TransactionBuilder.
    ///
    /// # Arguments
    ///
    /// * `rpc_client` - A shared `Arc<RpcClient>` for communicating with the Solana cluster.
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self { rpc_client }
    }

    /// Submits a fully signed transaction to the Solana network.
    ///
    /// This is the final step in the remote signing flow. After a client signs
    /// the transaction prepared by one of the `prepare_` methods, the signed
    /// transaction is sent back to the server and submitted via this method.
    ///
    /// # Arguments
    ///
    /// * `transaction` - A `Transaction` object that has already been signed.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Signature` of the confirmed transaction.
    pub async fn submit_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Signature, ClientError> {
        self.rpc_client
            .send_and_confirm_transaction(transaction)
            .await
    }

    /// A private helper function to create a transaction from a single instruction.
    ///
    /// This function encapsulates the boilerplate of fetching the latest blockhash
    /// and creating a new transaction with a payer.
    async fn create_transaction(
        &self,
        payer: &Pubkey,
        instruction: Instruction,
    ) -> Result<Transaction, ClientError> {
        let latest_blockhash = self.rpc_client.get_latest_blockhash().await?;
        let mut tx = Transaction::new_with_payer(&[instruction], Some(payer));
        tx.message.recent_blockhash = latest_blockhash;
        Ok(tx)
    }

    // --- Admin Transaction Preparations ---

    /// Prepares an `admin_register_profile` transaction.
    ///
    /// # Arguments
    ///
    /// * `authority` - The public key of the admin who will sign the transaction.
    /// * `communication_pubkey` - The public key for secure off-chain communication.
    pub async fn prepare_admin_register_profile(
        &self,
        authority: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_bridge_program::ID);

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminRegisterProfile {
                authority,
                admin_profile: admin_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::AdminRegisterProfile {
                communication_pubkey,
            }
            .data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares an `admin_update_comm_key` transaction.
    pub async fn prepare_admin_update_comm_key(
        &self,
        authority: Pubkey,
        new_key: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_bridge_program::ID);

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminUpdateCommKey {
                authority,
                admin_profile: admin_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminUpdateCommKey { new_key }.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares an `admin_update_prices` transaction.
    pub async fn prepare_admin_update_prices(
        &self,
        authority: Pubkey,
        new_prices: Vec<PriceEntry>,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_bridge_program::ID);

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminUpdatePrices {
                authority,
                admin_profile: admin_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::AdminUpdatePrices {
                args: UpdatePricesArgs { new_prices },
            }
            .data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares an `admin_withdraw` transaction.
    pub async fn prepare_admin_withdraw(
        &self,
        authority: Pubkey,
        amount: u64,
        destination: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_bridge_program::ID);

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminWithdraw {
                authority,
                admin_profile: admin_pda,
                destination,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::AdminWithdraw { amount }.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares an `admin_close_profile` transaction.
    pub async fn prepare_admin_close_profile(
        &self,
        authority: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_bridge_program::ID);

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminCloseProfile {
                authority,
                admin_profile: admin_pda,
            }
            .to_account_metas(None),
            data: instruction::AdminCloseProfile {}.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares an `admin_dispatch_command` transaction.
    pub async fn prepare_admin_dispatch_command(
        &self,
        authority: Pubkey,
        target_user_profile_pda: Pubkey,
        command_id: u64,
        payload: Vec<u8>,
    ) -> Result<Transaction, ClientError> {
        let (admin_pda, _) =
            Pubkey::find_program_address(&[b"admin", authority.as_ref()], &w3b2_bridge_program::ID);

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::AdminDispatchCommand {
                admin_authority: authority,
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

        self.create_transaction(&authority, ix).await
    }

    // --- User Transaction Preparations ---

    /// Prepares a `user_create_profile` transaction.
    pub async fn prepare_user_create_profile(
        &self,
        authority: Pubkey,
        target_admin_pda: Pubkey,
        communication_pubkey: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), target_admin_pda.as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserCreateProfile {
                authority,
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

        self.create_transaction(&authority, ix).await
    }

    /// Prepares a `user_update_comm_key` transaction.
    pub async fn prepare_user_update_comm_key(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        new_key: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserUpdateCommKey {
                authority,
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::UserUpdateCommKey { new_key }.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares a `user_deposit` transaction.
    pub async fn prepare_user_deposit(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        amount: u64,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserDeposit {
                authority,
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::UserDeposit { amount }.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares a `user_withdraw` transaction.
    pub async fn prepare_user_withdraw(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        amount: u64,
        destination: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserWithdraw {
                authority,
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
                destination,
                system_program: solana_sdk::system_program::id(),
            }
            .to_account_metas(None),
            data: instruction::UserWithdraw { amount }.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    /// Prepares a `user_close_profile` transaction.
    pub async fn prepare_user_close_profile(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserCloseProfile {
                authority,
                user_profile: user_pda,
                admin_profile: admin_profile_pda,
            }
            .to_account_metas(None),
            data: instruction::UserCloseProfile {}.data(),
        };

        self.create_transaction(&authority, ix).await
    }

    // --- Operational Transaction Preparations ---

    /// Prepares a `user_dispatch_command` transaction.
    pub async fn prepare_user_dispatch_command(
        &self,
        authority: Pubkey,
        admin_profile_pda: Pubkey,
        command_id: u16,
        payload: Vec<u8>,
    ) -> Result<Transaction, ClientError> {
        let (user_pda, _) = Pubkey::find_program_address(
            &[b"user", authority.as_ref(), admin_profile_pda.as_ref()],
            &w3b2_bridge_program::ID,
        );

        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::UserDispatchCommand {
                authority,
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

        self.create_transaction(&authority, ix).await
    }

    /// Prepares a `log_action` transaction.
    pub async fn prepare_log_action(
        &self,
        authority: Pubkey,
        session_id: u64,
        action_code: u16,
    ) -> Result<Transaction, ClientError> {
        let ix = Instruction {
            program_id: w3b2_bridge_program::ID,
            accounts: accounts::LogAction { authority }.to_account_metas(None),
            data: instruction::LogAction {
                session_id,
                action_code,
            }
            .data(),
        };

        self.create_transaction(&authority, ix).await
    }
}
