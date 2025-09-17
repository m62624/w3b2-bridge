use anyhow::Context;
use anyhow::Result;
use chrono::Local;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use tokio_stream::StreamExt;
use tonic::{transport::Server, Request, Response, Status};
use tracing::Level;
use w3b2_bridge_program::protocol::CommandMode;
use w3b2_connector::events::BridgeEvent as Event;
use w3b2_connector::{Storage, SyncConfig, Synchronizer};

pub mod bridge_proto {
    tonic::include_proto!("bridge");
}

use bridge_proto::bridge_service_server::{BridgeService, BridgeServiceServer};
use bridge_proto::{BridgeEvent, Empty};

#[derive(Parser, Debug)]
#[command(
    name = "w3b2-connector",
    version,
    about = "Bridge connector gRPC server"
)]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[derive(Clone)]
pub struct BridgeServer {
    storage: Storage,
    config: SyncConfig,
}

fn convert_event_to_proto(event: Event) -> BridgeEvent {
    match event {
        Event::AdminRegistered(e) => BridgeEvent {
            event: Some(bridge_proto::bridge_event::Event::AdminRegistered(
                bridge_proto::AdminRegistered {
                    admin: e.admin.to_string(),
                    initial_funding: e.initial_funding,
                    ts: e.ts,
                },
            )),
        },
        Event::UserRegistered(e) => BridgeEvent {
            event: Some(bridge_proto::bridge_event::Event::UserRegistered(
                bridge_proto::UserRegistered {
                    user: e.user.to_string(),
                    initial_balance: e.initial_balance,
                    ts: e.ts,
                },
            )),
        },
        Event::AdminDeactivated(e) => BridgeEvent {
            event: Some(bridge_proto::bridge_event::Event::AdminDeactivated(
                bridge_proto::AdminDeactivated {
                    admin: e.admin.to_string(),
                    ts: e.ts,
                },
            )),
        },
        Event::UserDeactivated(e) => BridgeEvent {
            event: Some(bridge_proto::bridge_event::Event::UserDeactivated(
                bridge_proto::UserDeactivated {
                    user: e.user.to_string(),
                    ts: e.ts,
                },
            )),
        },
        Event::FundingRequested(e) => BridgeEvent {
            event: Some(bridge_proto::bridge_event::Event::FundingRequested(
                bridge_proto::FundingRequested {
                    user_wallet: e.user_wallet.to_string(),
                    target_admin: e.target_admin.to_string(),
                    amount: e.amount,
                    ts: e.ts,
                },
            )),
        },
        Event::FundingApproved(e) => BridgeEvent {
            event: Some(bridge_proto::bridge_event::Event::FundingApproved(
                bridge_proto::FundingApproved {
                    user_wallet: e.user_wallet.to_string(),
                    approved_by: e.approved_by.to_string(),
                    amount: e.amount,
                    ts: e.ts,
                },
            )),
        },
        Event::CommandEvent(e) => BridgeEvent {
            event: Some(bridge_proto::bridge_event::Event::CommandEvent(
                bridge_proto::CommandEvent {
                    sender: e.sender.to_string(),
                    target: e.target.to_string(),
                    command_id: e.command_id,
                    mode: match e.mode {
                        CommandMode::RequestResponse => {
                            bridge_proto::CommandMode::RequestResponse as i32
                        }
                        CommandMode::OneWay => bridge_proto::CommandMode::OneWay as i32,
                    },
                    payload: e.payload,
                    ts: e.ts,
                },
            )),
        },
        Event::Unknown => BridgeEvent { event: None },
    }
}

#[tonic::async_trait]
impl BridgeService for BridgeServer {
    type StreamEventsStream = tokio_stream::wrappers::ReceiverStream<Result<BridgeEvent, Status>>;

    async fn stream_events(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::StreamEventsStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let storage = self.storage.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut event_stream = Synchronizer::builder()
                .with_config(config)
                .with_storage(storage)
                .start()
                .await
                .unwrap();

            while let Some(event) = event_stream.next().await {
                let proto_event = convert_event_to_proto(event);
                if proto_event.event.is_none() {
                    continue;
                }
                if tx.send(Ok(proto_event)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config: SyncConfig = if cli.config.exists() {
        let content = fs::read_to_string(&cli.config)
            .with_context(|| format!("Failed to read {:?}", cli.config))?;
        toml::from_str(&content).with_context(|| format!("Failed to parse {:?}", cli.config))?
    } else {
        let default_config = SyncConfig::default();
        fs::write(&cli.config, toml::to_string_pretty(&default_config)?)?;
        default_config
    };

    let logs_path = PathBuf::from(&config.data_dir).join(&config.log_dir);
    fs::create_dir_all(&logs_path)?;
    let date = Local::now().format("%Y-%m-%d").to_string();
    let log_file_path = logs_path.join(format!("w3b2-{}.log", date));
    let file = fs::File::create(log_file_path)?;

    let level = match config.log_level.to_uppercase().as_str() {
        "TRACE" => Level::TRACE,
        "DEBUG" => Level::DEBUG,
        "INFO" => Level::INFO,
        "WARN" => Level::WARN,
        "ERROR" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = tracing_subscriber::fmt()
        .with_writer(file)
        .with_max_level(level);

    if config.log_format.to_lowercase() == "json" {
        subscriber.json().init();
    } else {
        subscriber.init();
    }

    tracing::info!("{:#?}", config);
    let addr = format!("{}:{}", config.host, config.port).parse()?;
    let storage = Storage::new(&config.data_dir)?;
    let bridge_service = BridgeServer { storage, config };

    tracing::info!("gRPC server listening on {}", addr);

    Server::builder()
        .add_service(BridgeServiceServer::new(bridge_service))
        .serve(addr)
        .await?;

    Ok(())
}
