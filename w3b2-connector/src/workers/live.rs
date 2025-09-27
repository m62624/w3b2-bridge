// w3b2-connector/src/live.rs

use anyhow::Result;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_response::Response,
};
use solana_sdk::commitment_config::CommitmentConfig;
use tokio_stream::StreamExt;

use crate::workers::WorkerContext;

pub struct LiveWorker {
    ctx: WorkerContext,
}

impl LiveWorker {
    pub fn new(ctx: WorkerContext) -> Self {
        Self { ctx }
    }

    /// Subscribes to new logs via WebSocket and processes them in real-time.
    pub async fn run(&self) -> Result<()> {
        let client = PubsubClient::new(&self.ctx.config.solana.ws_url).await?;

        let (mut stream, _) = client
            .logs_subscribe(
                RpcTransactionLogsFilter::Mentions(vec![w3b2_bridge_program::ID.to_string()]),
                RpcTransactionLogsConfig {
                    commitment: Some(CommitmentConfig {
                        commitment: self.ctx.config.solana.commitment,
                    }),
                },
            )
            .await?;

        tracing::info!("Live worker connected to WebSocket and listening for logs.");

        while let Some(msg) = stream.next().await {
            let Response { context, value } = msg;
            let slot = context.slot;

            if slot <= self.ctx.storage.get_last_slot().await? {
                continue;
            }

            for log in value.logs {
                if let Ok(event) = crate::events::try_parse_log(&log) {
                    if !matches!(event, crate::events::BridgeEvent::Unknown) {
                        tracing::info!("[LIVE] slot={} event={:?}", slot, event);
                        if self.ctx.event_sender.send(event).is_err() {
                            tracing::warn!("No active receivers for broadcast channel.");
                        }
                    }
                }
            }
            self.ctx
                .storage
                .set_sync_state(slot, &value.signature)
                .await?;
        }
        Ok(())
    }
}
