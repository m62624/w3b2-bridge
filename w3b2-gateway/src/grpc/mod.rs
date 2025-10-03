// File: w3b2-gateway/src/grpc.rs
mod conversions;
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};
use std::str::FromStr;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use w3b2_connector::{
    Accounts::PriceEntry,
    client::TransactionBuilder,
    listener::{AdminListener, UserListener},
    workers::EventManager,
};

use base64::{Engine as _, engine::general_purpose};

use crate::{
    config,
    grpc::proto::w3b2::bridge::gateway::{
        AdminEventStream, BridgeEvent, ListenRequest, PrepareAdminCloseProfileRequest,
        PrepareAdminDispatchCommandRequest, PrepareAdminRegisterProfileRequest,
        PrepareAdminUpdateCommKeyRequest, PrepareAdminUpdatePricesRequest,
        PrepareAdminWithdrawRequest, PrepareLogActionRequest, PrepareUserCloseProfileRequest,
        PrepareUserCreateProfileRequest, PrepareUserDepositRequest,
        PrepareUserDispatchCommandRequest, PrepareUserUpdateCommKeyRequest,
        PrepareUserWithdrawRequest, SubmitTransactionRequest, TransactionResponse,
        UnsignedTransactionResponse, UserEventStream, admin_event_stream, bridge_event,
        bridge_gateway_service_server::{BridgeGatewayService, BridgeGatewayServiceServer},
        user_event_stream,
    },
};

pub mod proto {
    pub mod w3b2 {
        pub mod bridge {
            pub mod gateway {
                tonic::include_proto!("w3b2.bridge.gateway");
            }
        }
    }
}

/// Shared application state for the gRPC server.
/// In this non-custodial model, it only needs the RpcClient.
#[derive(Clone)]
pub struct AppState {
    pub rpc_client: Arc<RpcClient>,
    pub event_manager: Arc<EventManager>,
}

pub struct GatewayServer {
    state: AppState,
}

impl GatewayServer {
    /// Creates a new instance of the GatewayServer.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl BridgeGatewayService for GatewayServer {
    // ===================================================================
    // == Event Streaming Implementations
    // ===================================================================

    type ListenAsUserStream = ReceiverStream<Result<UserEventStream, Status>>;

    async fn listen_as_user(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::ListenAsUserStream>, Status> {
        let req = request.into_inner();
        let pubkey = Pubkey::from_str(&req.pubkey_to_follow)
            .map_err(|_| Status::invalid_argument("Invalid pubkey_to_follow format"))?;

        let user_listener: UserListener =
            self.state.event_manager.listen_as_user(pubkey, 1024).await;

        let (mut personal_rx, mut interactions_rx) = user_listener.into_parts();

        let (tx, rx) = tokio::sync::mpsc::channel(1024);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = personal_rx.recv() => {
                        let stream_msg = UserEventStream {
                            event_category: Some(user_event_stream::EventCategory::PersonalEvent(event.into())),
                        };
                        if tx.send(Ok(stream_msg)).await.is_err() { break; }
                    },
                    Some(event) = interactions_rx.recv() => {
                        let stream_msg = UserEventStream {
                            event_category: Some(user_event_stream::EventCategory::ServiceInteractionEvent(event.into())),
                        };
                        if tx.send(Ok(stream_msg)).await.is_err() { break; }
                    },
                    else => { break; }
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(stream))
    }

    type ListenAsAdminStream = ReceiverStream<Result<AdminEventStream, Status>>;

    async fn listen_as_admin(
        &self,
        request: Request<ListenRequest>,
    ) -> Result<Response<Self::ListenAsAdminStream>, Status> {
        let req = request.into_inner();
        let pubkey = Pubkey::from_str(&req.pubkey_to_follow)
            .map_err(|_| Status::invalid_argument("Invalid pubkey_to_follow format"))?;

        let admin_listener: AdminListener =
            self.state.event_manager.listen_as_admin(pubkey, 1024).await;

        let (mut personal_rx, mut commands_rx, mut new_users_rx) = admin_listener.into_parts();

        let (tx, rx) = tokio::sync::mpsc::channel(1024);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = personal_rx.recv() => {
                        // This part is simple: convert the whole enum.
                        let stream_msg = AdminEventStream {
                             event_category: Some(admin_event_stream::EventCategory::PersonalEvent(event.into())),
                        };
                        if tx.send(Ok(stream_msg)).await.is_err() { break; }
                    },
                    Some(event) = commands_rx.recv() => {

                        let proto_event: BridgeEvent = event.into();

                        if let Some(bridge_event::Event::UserCommandDispatched(proto_specific_event)) = proto_event.event {
                            let stream_msg = AdminEventStream {
                                event_category: Some(admin_event_stream::EventCategory::IncomingUserCommand(proto_specific_event)),
                            };
                            if tx.send(Ok(stream_msg)).await.is_err() { break; }
                        }
                    },
                    Some(event) = new_users_rx.recv() => {

                        let proto_event: BridgeEvent = event.into();
                        if let Some(bridge_event::Event::UserProfileCreated(proto_specific_event)) = proto_event.event {
                           let stream_msg = AdminEventStream {
                               event_category: Some(admin_event_stream::EventCategory::NewUserProfile(proto_specific_event)),
                           };
                           if tx.send(Ok(stream_msg)).await.is_err() { break; }
                        }
                    },
                    else => { break; }
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(stream))
    }

    // --- Admin Method Preparations ---

    async fn prepare_admin_register_profile(
        &self,
        request: Request<PrepareAdminRegisterProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let communication_pubkey = Pubkey::from_str(&req.communication_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid communication_pubkey"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_admin_register_profile(authority, communication_pubkey)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_admin_update_comm_key(
        &self,
        request: Request<PrepareAdminUpdateCommKeyRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let new_key = Pubkey::from_str(&req.new_key)
            .map_err(|_| Status::invalid_argument("Invalid new_key"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_admin_update_comm_key(authority, new_key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_admin_update_prices(
        &self,
        request: Request<PrepareAdminUpdatePricesRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;

        let new_prices = req
            .new_prices
            .into_iter()
            .map(|p| PriceEntry {
                command_id: p.command_id as u16,
                price: p.price,
            })
            .collect();

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_admin_update_prices(authority, new_prices)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_admin_withdraw(
        &self,
        request: Request<PrepareAdminWithdrawRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let destination = Pubkey::from_str(&req.destination)
            .map_err(|_| Status::invalid_argument("Invalid destination"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_admin_withdraw(authority, req.amount, destination)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_admin_close_profile(
        &self,
        request: Request<PrepareAdminCloseProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_admin_close_profile(authority)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_admin_dispatch_command(
        &self,
        request: Request<PrepareAdminDispatchCommandRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let target_user_profile_pda = Pubkey::from_str(&req.target_user_profile_pda)
            .map_err(|_| Status::invalid_argument("Invalid target_user_profile_pda"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_admin_dispatch_command(
                authority,
                target_user_profile_pda,
                req.command_id,
                req.payload,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    // --- User Method Preparations ---

    async fn prepare_user_create_profile(
        &self,
        request: Request<PrepareUserCreateProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let target_admin_pda = Pubkey::from_str(&req.target_admin_pda)
            .map_err(|_| Status::invalid_argument("Invalid target_admin_pda"))?;
        let communication_pubkey = Pubkey::from_str(&req.communication_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid communication_pubkey"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_user_create_profile(authority, target_admin_pda, communication_pubkey)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_user_update_comm_key(
        &self,
        request: Request<PrepareUserUpdateCommKeyRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let admin_profile_pda = Pubkey::from_str(&req.admin_profile_pda)
            .map_err(|_| Status::invalid_argument("Invalid admin_profile_pda"))?;
        let new_key = Pubkey::from_str(&req.new_key)
            .map_err(|_| Status::invalid_argument("Invalid new_key"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_user_update_comm_key(authority, admin_profile_pda, new_key)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_user_deposit(
        &self,
        request: Request<PrepareUserDepositRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let admin_profile_pda = Pubkey::from_str(&req.admin_profile_pda)
            .map_err(|_| Status::invalid_argument("Invalid admin_profile_pda"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_user_deposit(authority, admin_profile_pda, req.amount)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_user_withdraw(
        &self,
        request: Request<PrepareUserWithdrawRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let admin_profile_pda = Pubkey::from_str(&req.admin_profile_pda)
            .map_err(|_| Status::invalid_argument("Invalid admin_profile_pda"))?;
        let destination = Pubkey::from_str(&req.destination)
            .map_err(|_| Status::invalid_argument("Invalid destination"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_user_withdraw(authority, admin_profile_pda, req.amount, destination)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_user_close_profile(
        &self,
        request: Request<PrepareUserCloseProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let admin_profile_pda = Pubkey::from_str(&req.admin_profile_pda)
            .map_err(|_| Status::invalid_argument("Invalid admin_profile_pda"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_user_close_profile(authority, admin_profile_pda)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    async fn prepare_user_dispatch_command(
        &self,
        request: Request<PrepareUserDispatchCommandRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;
        let admin_profile_pda = Pubkey::from_str(&req.admin_profile_pda)
            .map_err(|_| Status::invalid_argument("Invalid admin_profile_pda"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_user_dispatch_command(
                authority,
                admin_profile_pda,
                req.command_id as u16,
                req.payload,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    // --- Operational Method Preparations ---

    async fn prepare_log_action(
        &self,
        request: Request<PrepareLogActionRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let req = request.into_inner();
        let authority = Pubkey::from_str(&req.authority_pubkey)
            .map_err(|_| Status::invalid_argument("Invalid authority_pubkey"))?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let transaction = builder
            .prepare_log_action(authority, req.session_id, req.action_code as u16)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let serialized_tx =
            bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                .map_err(|e| Status::internal(format!("Serialization failed: {}", e)))?;
        let unsigned_tx_base64 = general_purpose::STANDARD.encode(serialized_tx);

        Ok(Response::new(UnsignedTransactionResponse {
            unsigned_tx_base64,
        }))
    }

    // --- Transaction Submission ---

    /// Receives a transaction signed by a client and submits it to the Solana network.
    async fn submit_transaction(
        &self,
        request: Request<SubmitTransactionRequest>,
    ) -> Result<Response<TransactionResponse>, Status> {
        let req = request.into_inner();

        let tx_bytes = general_purpose::STANDARD
            .decode(&req.signed_tx_base64)
            .map_err(|e| {
                Status::invalid_argument(format!("Invalid base64 for transaction: {}", e))
            })?;

        let (transaction, _len): (Transaction, usize) =
            bincode::serde::borrow_decode_from_slice(&tx_bytes, bincode::config::standard())
                .map_err(|e| {
                    Status::invalid_argument(format!("Failed to deserialize transaction: {}", e))
                })?;

        let builder = TransactionBuilder::new(self.state.rpc_client.clone());
        let signature = builder
            .submit_transaction(&transaction)
            .await
            .map_err(|e| Status::internal(format!("Failed to send transaction: {}", e)))?;

        Ok(Response::new(TransactionResponse {
            signature: signature.to_string(),
        }))
    }
}

// /// The main entry point to start the gRPC server.
// pub async fn start(config: &config::GatewayConfig) -> Result<()> {
//     let addr = format!("{}:{}", config.gateway.grpc.host, config.gateway.grpc.port).parse()?;

//     // Create a single, shared RpcClient instance.
//     let rpc_client = Arc::new(RpcClient::new(config.connector.solana.rpc_url.clone()));

//     // --- NEW: Initialize EventManager and its background tasks ---
//     // Note: The storage dependency needs to be created here.
//     // For now, let's assume a simple in-memory or sled-based storage.
//     // let storage = Arc::new(SledStorage::new(...)); // You need to initialize this
//     let event_manager = Arc::new(EventManager::new(
//         Arc::new(config.connector.clone()),
//         rpc_client.clone(),
//         storage, // Pass the storage implementation
//         1024,
//         128,
//     ));

//     // Spawn the EventManager's background services (Synchronizer and Dispatcher)
//     let em_clone = event_manager.clone();
//     tokio::spawn(async move {
//         em_clone.run().await;
//     });
//     // --- END NEW ---

//     // The AppState now holds the EventManager instance.
//     let app_state = AppState {
//         rpc_client,
//         event_manager,
//     };

//     // Instantiate the server.
//     let gateway_server = GatewayServer::new(app_state);

//     tracing::info!(
//         "Non-Custodial gRPC Gateway with Event Streaming listening on {}",
//         addr
//     );

//     // Build and run the Tonic server with our single service.
//     Server::builder()
//         .add_service(BridgeGatewayServiceServer::new(gateway_server))
//         .serve(addr)
//         .await?;

//     Ok(())
// }
