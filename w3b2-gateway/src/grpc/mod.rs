mod conversions;
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};
use std::str::FromStr;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, transport::Server};
use w3b2_connector::{
    Accounts::PriceEntry,
    client::TransactionBuilder,
    listener::AdminListener,
    workers::{EventManager, EventManagerHandle},
};

use crate::{
    config::GatewayConfig,
    error::GatewayError,
    grpc::proto::w3b2::bridge::gateway::{
        AdminEventStream, BridgeEvent, ListenAsAdminRequest, ListenAsUserRequest,
        PrepareAdminCloseProfileRequest, PrepareAdminDispatchCommandRequest,
        PrepareAdminRegisterProfileRequest, PrepareAdminUpdateCommKeyRequest,
        PrepareAdminUpdatePricesRequest, PrepareAdminWithdrawRequest, PrepareLogActionRequest,
        PrepareUserCloseProfileRequest, PrepareUserCreateProfileRequest, PrepareUserDepositRequest,
        PrepareUserDispatchCommandRequest, PrepareUserUpdateCommKeyRequest,
        PrepareUserWithdrawRequest, SubmitTransactionRequest, TransactionResponse,
        UnsignedTransactionResponse, UnsubscribeRequest, UserEventStream,
        admin_event_stream::EventCategory as AdminEventCategory,
        bridge_event,
        bridge_gateway_service_server::{BridgeGatewayService, BridgeGatewayServiceServer},
        user_event_stream::EventCategory as UserEventCategory,
    },
    storage::SledStorage,
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

#[derive(Clone)]
pub struct AppState {
    pub rpc_client: Arc<RpcClient>,
    pub event_manager: EventManagerHandle,
    pub config: Arc<GatewayConfig>,
}

/// gRPC server implementation.
pub struct GatewayServer {
    state: AppState,
}

impl GatewayServer {
    /// Create a new GatewayServer instance.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// The main entry point to start the gRPC server and all background services.
pub async fn start(config: &GatewayConfig) -> Result<EventManagerHandle> {
    // --- 1. Initialize dependencies ---
    let db = sled::open(&config.gateway.db_path)?;
    let storage = Arc::new(SledStorage::new(db));
    let addr = format!("{}:{}", config.gateway.grpc.host, config.gateway.grpc.port).parse()?;
    let rpc_client = Arc::new(RpcClient::new(config.connector.solana.rpc_url.clone()));

    // --- 2. Create and spawn the EventManager service ---

    // `EventManager::new` now returns the runner and its handle.
    let (event_manager_runner, event_manager_handle) = EventManager::new(
        Arc::new(config.connector.clone()),
        rpc_client.clone(),
        storage,
        config.gateway.streaming.broadcast_capacity,
        config.gateway.streaming.command_capacity,
    );

    tokio::spawn(event_manager_runner.run());

    // --- 3. Set up the gRPC server state ---

    // Clone the handle for the gRPC server state. The original will be returned.
    let handle_for_server = event_manager_handle.clone();

    // Create the shared state, storing the lightweight `handle` for the RPCs to use.
    let app_state = AppState {
        rpc_client,
        event_manager: handle_for_server, // Store the cloned handle
        config: Arc::new(config.clone()),
    };

    let gateway_server = GatewayServer::new(app_state);

    tracing::info!(
        "Non-Custodial gRPC Gateway with Event Streaming listening on {}",
        addr
    );

    // --- 4. Start the gRPC server ---
    let grpc_server =
        Server::builder().add_service(BridgeGatewayServiceServer::new(gateway_server));

    tokio::spawn(async move {
        if let Err(e) = grpc_server.serve(addr).await {
            tracing::error!("gRPC server failed: {}", e);
        }
    });

    Ok(event_manager_handle)
}

// helper: parse a Pubkey returning GatewayError
fn parse_pubkey(s: &str) -> Result<Pubkey, GatewayError> {
    Pubkey::from_str(s).map_err(GatewayError::from)
}

#[tonic::async_trait]
impl BridgeGatewayService for GatewayServer {
    type ListenAsUserStream = ReceiverStream<Result<UserEventStream, Status>>;

    async fn listen_as_user(
        &self,
        request: Request<ListenAsUserRequest>,
    ) -> Result<Response<Self::ListenAsUserStream>, Status> {
        let result: Result<Response<Self::ListenAsUserStream>, GatewayError> = (async {
            tracing::info!(
                "Received ListenAsUser request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();

            // Use config-driven capacities.
            let listener_capacity = self.state.config.gateway.streaming.listener_channel_capacity;
            let service_listener_capacity = self.state.config.gateway.streaming.service_listener_capacity;
            let output_capacity = self.state.config.gateway.streaming.output_stream_capacity;

            let pubkey = parse_pubkey(&req.user_pubkey)?;

            tracing::debug!("Creating user listener for pubkey: {}", pubkey);
            let user_listener = self.state.event_manager.listen_as_user(pubkey, listener_capacity).await;

            let mut specific_service_rxs = Vec::new();
            for pda_str in req.specific_services_to_follow {
                let pda = parse_pubkey(&pda_str)?;
                tracing::debug!("Subscribing user {} to specific service PDA: {}", pubkey, pda);
                specific_service_rxs.push(user_listener.listen_for_service(pda, service_listener_capacity));
            }

            let (mut personal_rx, mut interactions_rx) = user_listener.into_parts();
            let (tx, rx) = tokio::sync::mpsc::channel(output_capacity);
            let event_manager = self.state.event_manager.clone();

            tokio::spawn(async move {
                // Task for merging specific service listeners
                 let (specific_tx, mut specific_rx_merged) = tokio::sync::mpsc::channel(output_capacity);

            for mut service_rx in specific_service_rxs {
                let inner_tx = specific_tx.clone();
                tokio::spawn(async move {
                    while let Some(event) = service_rx.recv().await {
                        if inner_tx.send(event).await.is_err() { break; }
                    }
                });
            }
            drop(specific_tx);

                loop {
                    tokio::select! {
                        biased;

                        Some(event) = personal_rx.recv() => {
                            let msg = UserEventStream { event_category: Some(UserEventCategory::PersonalEvent(event.into())) };
                            tracing::debug!("Forwarding personal event to user {}: {:?}", pubkey, msg);
                            if tx.send(Ok(msg)).await.is_err() { break; }
                        },
                        Some(event) = interactions_rx.recv() => {
                            let msg = UserEventStream { event_category: Some(UserEventCategory::ServiceInteractionEvent(event.into())) };
                            tracing::debug!("Forwarding service interaction event to user {}: {:?}", pubkey, msg);
                            if tx.send(Ok(msg)).await.is_err() { break; }
                        },
                        Some(event) = specific_rx_merged.recv() => {
                            let msg = UserEventStream { event_category: Some(UserEventCategory::ServiceSpecificEvent(event.into())) };
                            tracing::debug!("Forwarding service-specific event to user {}: {:?}", pubkey, msg);
                            if tx.send(Ok(msg)).await.is_err() { break; }
                        },
                        else => { break; }
                    }
                }
                event_manager.unsubscribe(pubkey).await;
            });

            Ok(Response::new(ReceiverStream::new(rx)))
        })
        .await;

        result.map_err(Status::from)
    }

    type ListenAsAdminStream = ReceiverStream<Result<AdminEventStream, Status>>;

    async fn listen_as_admin(
        &self,
        request: Request<ListenAsAdminRequest>,
    ) -> Result<Response<Self::ListenAsAdminStream>, Status> {
        let result: Result<Response<Self::ListenAsAdminStream>, GatewayError> = (async {
            tracing::info!(
                "Received ListenAsAdmin request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();

            let listener_capacity = self.state.config.gateway.streaming.listener_channel_capacity;
            let output_capacity = self.state.config.gateway.streaming.output_stream_capacity;

            let pubkey = parse_pubkey(&req.admin_pubkey)?;
            let admin_listener: AdminListener = self.state.event_manager.listen_as_admin(pubkey, listener_capacity).await;
            tracing::debug!("Created admin listener for pubkey: {}", pubkey);

            let (mut personal_rx, mut commands_rx, mut new_users_rx) = admin_listener.into_parts();
            let (tx, rx) = tokio::sync::mpsc::channel(output_capacity);
            let event_manager = self.state.event_manager.clone();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        Some(event) = personal_rx.recv() => {
                            let stream_msg = AdminEventStream {
                                 event_category: Some(AdminEventCategory::PersonalEvent(event.into())),
                            };
                            tracing::debug!("Forwarding personal event to admin {}: {:?}", pubkey, stream_msg);
                            if tx.send(Ok(stream_msg)).await.is_err() { break; }
                        },
                        Some(event) = commands_rx.recv() => {
                            let proto_event: BridgeEvent = event.into();
                            if let Some(bridge_event::Event::UserCommandDispatched(proto_specific_event)) = proto_event.event {
                                let stream_msg = AdminEventStream {
                                    event_category: Some(AdminEventCategory::IncomingUserCommand(proto_specific_event)),
                                };
                                tracing::debug!("Forwarding incoming user command to admin {}: {:?}", pubkey, stream_msg);
                                if tx.send(Ok(stream_msg)).await.is_err() { break; }
                            }
                        },
                        Some(event) = new_users_rx.recv() => {
                            let proto_event: BridgeEvent = event.into();
                            if let Some(bridge_event::Event::UserProfileCreated(proto_specific_event)) = proto_event.event {
                               let stream_msg = AdminEventStream {
                                   event_category: Some(AdminEventCategory::NewUserProfile(proto_specific_event)),
                               };
                               tracing::debug!("Forwarding new user profile event to admin {}: {:?}", pubkey, stream_msg);
                               if tx.send(Ok(stream_msg)).await.is_err() { break; }
                            }
                        },
                        else => { break; }
                    }
                }
                event_manager.unsubscribe(pubkey).await;
            });

            Ok(Response::new(ReceiverStream::new(rx)))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn unsubscribe(
        &self,
        request: Request<UnsubscribeRequest>,
    ) -> Result<Response<()>, Status> {
        let result: Result<Response<()>, GatewayError> = (async {
            tracing::info!("Received Unsubscribe request: {:?}", request.get_ref());

            let req = request.into_inner();
            let pubkey = parse_pubkey(&req.pubkey_to_unsubscribe)?;
            tracing::info!("Received explicit unsubscribe request for {}", pubkey);
            self.state.event_manager.unsubscribe(pubkey).await;
            Ok(Response::new(()))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_admin_register_profile(
        &self,
        request: Request<PrepareAdminRegisterProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminRegisterProfile request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let communication_pubkey = parse_pubkey(&req.communication_pubkey)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_admin_register_profile(authority, communication_pubkey)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_register_profile tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_admin_update_comm_key(
        &self,
        request: Request<PrepareAdminUpdateCommKeyRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminUpdateCommKey request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let new_key = parse_pubkey(&req.new_key)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_admin_update_comm_key(authority, new_key)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_update_comm_key tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_admin_update_prices(
        &self,
        request: Request<PrepareAdminUpdatePricesRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminUpdatePrices request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;

            let new_prices = req
                .new_prices
                .into_iter()
                .map(|p| PriceEntry {
                    command_id: p.command_id as u16,
                    price: p.price,
                })
                .collect::<Vec<PriceEntry>>();

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_admin_update_prices(authority, new_prices)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_update_prices tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_admin_withdraw(
        &self,
        request: Request<PrepareAdminWithdrawRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminWithdraw request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let destination = parse_pubkey(&req.destination)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_admin_withdraw(authority, req.amount, destination)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared admin_withdraw tx for authority {}", authority);

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_admin_close_profile(
        &self,
        request: Request<PrepareAdminCloseProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminCloseProfile request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_admin_close_profile(authority)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_close_profile tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_admin_dispatch_command(
        &self,
        request: Request<PrepareAdminDispatchCommandRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareAdminDispatchCommand request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let target_user_profile_pda = parse_pubkey(&req.target_user_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_admin_dispatch_command(
                    authority,
                    target_user_profile_pda,
                    req.command_id,
                    req.payload,
                )
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared admin_dispatch_command tx for authority {}",
                authority
            );

            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_user_create_profile(
        &self,
        request: Request<PrepareUserCreateProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserCreateProfile request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let target_admin_pda = parse_pubkey(&req.target_admin_pda)?;
            let communication_pubkey = parse_pubkey(&req.communication_pubkey)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_user_create_profile(authority, target_admin_pda, communication_pubkey)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared user_create_profile tx for authority {}",
                authority
            );
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_user_update_comm_key(
        &self,
        request: Request<PrepareUserUpdateCommKeyRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserUpdateCommKey request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;
            let new_key = parse_pubkey(&req.new_key)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_user_update_comm_key(authority, admin_profile_pda, new_key)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared user_update_comm_key tx for authority {}",
                authority
            );
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_user_deposit(
        &self,
        request: Request<PrepareUserDepositRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserDeposit request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_user_deposit(authority, admin_profile_pda, req.amount)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared user_deposit tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_user_withdraw(
        &self,
        request: Request<PrepareUserWithdrawRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserWithdraw request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;
            let destination = parse_pubkey(&req.destination)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_user_withdraw(authority, admin_profile_pda, req.amount, destination)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared user_withdraw tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_user_close_profile(
        &self,
        request: Request<PrepareUserCloseProfileRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserCloseProfile request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_user_close_profile(authority, admin_profile_pda)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared user_close_profile tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_user_dispatch_command(
        &self,
        request: Request<PrepareUserDispatchCommandRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received PrepareUserDispatchCommand request: {:?}",
                request.get_ref()
            );

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;
            let admin_profile_pda = parse_pubkey(&req.admin_profile_pda)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_user_dispatch_command(
                    authority,
                    admin_profile_pda,
                    req.command_id as u16,
                    req.payload,
                )
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!(
                "Prepared user_dispatch_command tx for authority {}",
                authority
            );
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn prepare_log_action(
        &self,
        request: Request<PrepareLogActionRequest>,
    ) -> Result<Response<UnsignedTransactionResponse>, Status> {
        let result: Result<Response<UnsignedTransactionResponse>, GatewayError> = (async {
            tracing::info!("Received PrepareLogAction request: {:?}", request.get_ref());

            let req = request.into_inner();
            let authority = parse_pubkey(&req.authority_pubkey)?;

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let transaction = builder
                .prepare_log_action(authority, req.session_id, req.action_code as u16)
                .await
                .map_err(GatewayError::from)?;

            let unsigned_tx =
                bincode::serde::encode_to_vec(&transaction, bincode::config::standard())
                    .map_err(GatewayError::from)?;
            tracing::debug!("Prepared log_action tx for authority {}", authority);
            Ok(Response::new(UnsignedTransactionResponse { unsigned_tx }))
        })
        .await;

        result.map_err(Status::from)
    }

    async fn submit_transaction(
        &self,
        request: Request<SubmitTransactionRequest>,
    ) -> Result<Response<TransactionResponse>, Status> {
        let result: Result<Response<TransactionResponse>, GatewayError> = (async {
            tracing::info!(
                "Received SubmitTransaction request with {} bytes",
                request.get_ref().signed_tx.len()
            );

            let req = request.into_inner();
            let tx_bytes = req.signed_tx;

            let (transaction, _len): (Transaction, usize) =
                bincode::serde::borrow_decode_from_slice(
                    tx_bytes.as_slice(),
                    bincode::config::standard(),
                )
                .map_err(GatewayError::from)?;
            tracing::debug!("Deserialized transaction: {:?}", transaction);

            let builder = TransactionBuilder::new(self.state.rpc_client.clone());
            let signature = builder
                .submit_transaction(&transaction)
                .await
                .map_err(GatewayError::from)?;
            tracing::info!("Submitted transaction, signature: {}", signature);

            Ok(Response::new(TransactionResponse {
                signature: signature.to_string(),
            }))
        })
        .await;

        result.map_err(Status::from)
    }
}
