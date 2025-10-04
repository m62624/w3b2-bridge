use solana_client::client_error::ClientError;
use solana_sdk::pubkey::ParsePubkeyError;
use thiserror::Error;
use tonic::Status;

/// Defines the primary error types for the gRPC gateway.
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Internal connector error: {0}")]
    Connector(#[from] ClientError),

    #[error("Serialization failed: {0}")]
    Serialization(#[from] bincode::error::EncodeError),

    #[error("Deserialization failed: {0}")]
    Deserialization(#[from] bincode::error::DecodeError),

    #[error("Invalid Base64: {0}")]
    Base64(#[from] base64::DecodeError),
}

/// Allows automatic conversion from our custom `GatewayError` into a `tonic::Status`.
/// This cleans up all the `.map_err()` calls in the gRPC handlers.
impl From<GatewayError> for Status {
    fn from(err: GatewayError) -> Self {
        match err {
            GatewayError::InvalidArgument(reason) => Status::invalid_argument(reason),
            GatewayError::Connector(e) => {
                Status::internal(format!("Blockchain client error: {}", e))
            }
            GatewayError::Serialization(e) => {
                Status::internal(format!("Data serialization error: {}", e))
            }
            GatewayError::Deserialization(e) => {
                Status::invalid_argument(format!("Invalid data format for deserialization: {}", e))
            }
            GatewayError::Base64(e) => {
                Status::invalid_argument(format!("Invalid Base64 encoding: {}", e))
            }
        }
    }
}

/// Helper implementation to convert Pubkey parsing errors into our custom error type.
impl From<ParsePubkeyError> for GatewayError {
    fn from(err: ParsePubkeyError) -> Self {
        GatewayError::InvalidArgument(format!("Invalid public key format: {}", err))
    }
}
