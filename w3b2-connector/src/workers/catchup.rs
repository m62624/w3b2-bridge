use crate::{
    events::{try_parse_log, BridgeEvent},
    workers::WorkerContext,
};
use anyhow::Result;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::{
    rpc_config::RpcTransactionConfig, rpc_response::RpcConfirmedTransactionStatusWithSignature,
};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;
use tokio::time::{sleep, Duration};

pub struct CatchupWorker {
    ctx: WorkerContext,
    program_id: solana_sdk::pubkey::Pubkey,
}

impl CatchupWorker {
    pub fn new(ctx: WorkerContext) -> Self {
        let program_id = w3b2_bridge_program::ID;
        Self { ctx, program_id }
    }

    /// Runs the main catch-up loop.
    /// In each iteration, it fetches new signatures and processes them.
    pub async fn run(&self) -> Result<()> {
        loop {
            // The logic is now broken into smaller, more manageable pieces.
            let signatures = self.fetch_new_signatures().await?;
            if !signatures.is_empty() {
                tracing::info!("Found {} new signatures to process.", signatures.len());
                self.process_signatures(signatures).await?;
            }

            let poll_interval = self.ctx.config.synchronizer.poll_interval_secs;
            sleep(Duration::from_secs(poll_interval)).await;
        }
    }

    /// Fetches signatures in pages until it finds the last one we processed.
    async fn fetch_new_signatures(
        &self,
    ) -> Result<Vec<RpcConfirmedTransactionStatusWithSignature>> {
        let mut before_sig: Option<Signature> = None;
        let last_known_sig = self.ctx.storage.get_last_sig().await?;
        let mut signatures_to_process = Vec::new();

        tracing::info!(
            "Starting catch-up from last known signature: {:?}",
            last_known_sig
        );

        'fetch_loop: loop {
            let sig_config = GetConfirmedSignaturesForAddress2Config {
                before: before_sig,
                until: None,
                limit: Some(self.ctx.config.synchronizer.max_signature_fetch),
                commitment: Some(CommitmentConfig {
                    commitment: self.ctx.config.solana.commitment,
                }),
            };

            let sigs = self
                .ctx
                .rpc_client
                .get_signatures_for_address_with_config(&self.program_id, sig_config)
                .await?;

            if sigs.is_empty() {
                break 'fetch_loop;
            }
            before_sig = sigs.last().and_then(|s| s.signature.parse().ok());

            if let Some(ref last_sig) = last_known_sig {
                if let Some(pos) = sigs.iter().position(|s| &s.signature == last_sig) {
                    signatures_to_process.extend_from_slice(&sigs[..pos]);
                    break 'fetch_loop;
                }
            }
            signatures_to_process.extend(sigs);
        }

        // Process from oldest to newest.
        signatures_to_process.reverse();
        Ok(signatures_to_process)
    }

    /// Iterates through a batch of signatures and processes each one individually.
    async fn process_signatures(
        &self,
        signatures: Vec<RpcConfirmedTransactionStatusWithSignature>,
    ) -> Result<()> {
        let current_slot = self.ctx.rpc_client.get_slot().await?;

        for sig_info in signatures {
            if let Some(max_depth) = self.ctx.config.synchronizer.max_catchup_depth {
                if sig_info.slot < current_slot.saturating_sub(max_depth) {
                    tracing::debug!(
                        "Skipping {} from slot {} due to max_catchup_depth",
                        sig_info.signature,
                        sig_info.slot
                    );
                    continue;
                }
            }

            self.process_one_transaction(&sig_info).await?;
        }
        Ok(())
    }

    /// Fetches a single transaction, parses its logs for events, and sends them.
    async fn process_one_transaction(
        &self,
        sig_info: &RpcConfirmedTransactionStatusWithSignature,
    ) -> Result<()> {
        let sig = sig_info.signature.parse::<Signature>()?;
        let tx_config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: Some(CommitmentConfig {
                commitment: self.ctx.config.solana.commitment,
            }),
            max_supported_transaction_version: Some(0),
        };

        match self
            .ctx
            .rpc_client
            .get_transaction_with_config(&sig, tx_config)
            .await
        {
            Ok(tx) => {
                if let Some(meta) = tx.transaction.meta {
                    if let solana_transaction_status::option_serializer::OptionSerializer::Some(
                        logs,
                    ) = meta.log_messages
                    {
                        for log in logs {
                            if let Ok(event) = try_parse_log(&log) {
                                if !matches!(event, BridgeEvent::Unknown) {
                                    if self.ctx.event_sender.send(event).is_err() {
                                        tracing::warn!(
                                            "No active receivers for broadcast channel."
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                self.ctx
                    .storage
                    .set_sync_state(tx.slot, &sig_info.signature)
                    .await?;
            }
            Err(e) => tracing::error!("Failed to get transaction {}: {}", sig, e),
        }
        Ok(())
    }
}
