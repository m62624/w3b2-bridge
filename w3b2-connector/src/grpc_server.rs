// w3b2-connector/src/grpc_server.rs

use crate::config::Config;
use crate::events::BridgeEvent as ConnectorEvent;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

// Подключаем и именуем сгенерированный Protobuf код
pub mod proto {
    tonic::include_proto!("bridge");
}

// Импортируем типы для удобства
use proto::{
    bridge_service_server::BridgeService, AdminCommKeyUpdated, AdminCommandDispatched,
    AdminFundsWithdrawn, AdminPricesUpdated, AdminProfileClosed, AdminProfileRegistered,
    BridgeEvent as ProtoEvent, Empty, OffChainActionLogged, PriceEntry, UserCommKeyUpdated,
    UserCommandDispatched, UserFundsDeposited, UserFundsWithdrawn, UserProfileClosed,
    UserProfileCreated,
};

pub struct BridgeServer {
    event_tx: broadcast::Sender<ConnectorEvent>,
}

impl BridgeServer {
    pub fn new(event_tx: broadcast::Sender<ConnectorEvent>) -> Self {
        Self { event_tx }
    }
}

/// Конвертирует внутреннее событие коннектора в Protobuf-сообщение для отправки клиенту.
fn convert_event_to_proto(event: ConnectorEvent) -> ProtoEvent {
    let event_oneof = match event {
        ConnectorEvent::AdminProfileRegistered(e) => {
            proto::bridge_event::Event::AdminProfileRegistered(AdminProfileRegistered {
                authority: e.authority.to_string(),
                communication_pubkey: e.communication_pubkey.to_string(),
                ts: e.ts,
            })
        }
        ConnectorEvent::AdminCommKeyUpdated(e) => {
            proto::bridge_event::Event::AdminCommKeyUpdated(AdminCommKeyUpdated {
                authority: e.authority.to_string(),
                new_comm_pubkey: e.new_comm_pubkey.to_string(),
                ts: e.ts,
            })
        }
        ConnectorEvent::AdminPricesUpdated(e) => {
            proto::bridge_event::Event::AdminPricesUpdated(AdminPricesUpdated {
                authority: e.authority.to_string(),
                new_prices: e
                    .new_prices
                    .into_iter()
                    .map(|(command_id, price)| PriceEntry {
                        command_id: command_id as u32,
                        price,
                    })
                    .collect(),
                ts: e.ts,
            })
        }
        ConnectorEvent::AdminFundsWithdrawn(e) => {
            proto::bridge_event::Event::AdminFundsWithdrawn(AdminFundsWithdrawn {
                authority: e.authority.to_string(),
                amount: e.amount,
                destination: e.destination.to_string(),
                ts: e.ts,
            })
        }
        ConnectorEvent::AdminProfileClosed(e) => {
            proto::bridge_event::Event::AdminProfileClosed(AdminProfileClosed {
                authority: e.authority.to_string(),
                ts: e.ts,
            })
        }
        ConnectorEvent::AdminCommandDispatched(e) => {
            proto::bridge_event::Event::AdminCommandDispatched(AdminCommandDispatched {
                sender: e.sender.to_string(),
                target_user_authority: e.target_user_authority.to_string(),
                command_id: e.command_id as u32,
                payload: e.payload,
                ts: e.ts,
            })
        }
        ConnectorEvent::UserProfileCreated(e) => {
            proto::bridge_event::Event::UserProfileCreated(UserProfileCreated {
                authority: e.authority.to_string(),
                target_admin: e.target_admin.to_string(),
                communication_pubkey: e.communication_pubkey.to_string(),
                ts: e.ts,
            })
        }
        ConnectorEvent::UserCommKeyUpdated(e) => {
            proto::bridge_event::Event::UserCommKeyUpdated(UserCommKeyUpdated {
                authority: e.authority.to_string(),
                new_comm_pubkey: e.new_comm_pubkey.to_string(),
                ts: e.ts,
            })
        }
        ConnectorEvent::UserFundsDeposited(e) => {
            proto::bridge_event::Event::UserFundsDeposited(UserFundsDeposited {
                authority: e.authority.to_string(),
                amount: e.amount,
                new_deposit_balance: e.new_deposit_balance,
                ts: e.ts,
            })
        }
        ConnectorEvent::UserFundsWithdrawn(e) => {
            proto::bridge_event::Event::UserFundsWithdrawn(UserFundsWithdrawn {
                authority: e.authority.to_string(),
                amount: e.amount,
                destination: e.destination.to_string(),
                new_deposit_balance: e.new_deposit_balance,
                ts: e.ts,
            })
        }
        ConnectorEvent::UserProfileClosed(e) => {
            proto::bridge_event::Event::UserProfileClosed(UserProfileClosed {
                authority: e.authority.to_string(),
                ts: e.ts,
            })
        }
        ConnectorEvent::UserCommandDispatched(e) => {
            proto::bridge_event::Event::UserCommandDispatched(UserCommandDispatched {
                sender: e.sender.to_string(),
                target_admin_authority: e.target_admin_authority.to_string(),
                command_id: e.command_id as u32,
                price_paid: e.price_paid,
                payload: e.payload,
                ts: e.ts,
            })
        }
        ConnectorEvent::OffChainActionLogged(e) => {
            proto::bridge_event::Event::OffChainActionLogged(OffChainActionLogged {
                actor: e.actor.to_string(),
                session_id: e.session_id,
                action_code: e.action_code as u32,
                ts: e.ts,
            })
        }
        ConnectorEvent::Unknown => return ProtoEvent { event: None },
    };
    ProtoEvent {
        event: Some(event_oneof),
    }
}

#[tonic::async_trait]
impl BridgeService for BridgeServer {
    type StreamEventsStream = ReceiverStream<Result<ProtoEvent, Status>>; // <-- ИСПРАВЛЕНО

    async fn stream_events(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::StreamEventsStream>, Status> {
        tracing::info!("New gRPC client connected for event streaming.");
        let (tx, rx) = mpsc::channel(128);

        // Создаем нового подписчика на наш broadcast-канал
        let mut event_rx = self.event_tx.subscribe();

        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        let proto_event = convert_event_to_proto(event);
                        if proto_event.event.is_some() {
                            if tx.send(Ok(proto_event)).await.is_err() {
                                tracing::info!("gRPC client disconnected.");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Broadcast channel error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx))) // <-- ИСПРАВЛЕНО
    }
}

/// Запускает gRPC сервер.
pub async fn start(config: Arc<Config>, event_tx: broadcast::Sender<ConnectorEvent>) -> Result<()> {
    let addr = format!("{}:{}", config.grpc_server.host, config.grpc_server.port).parse()?;
    let bridge_service = BridgeServer::new(event_tx);

    tracing::info!("gRPC server listening on {}", addr);
    Server::builder()
        .add_service(proto::bridge_service_server::BridgeServiceServer::new(
            bridge_service,
        ))
        .serve(addr)
        .await?;
    Ok(())
}
