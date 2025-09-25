// w3b2-bridge-program/src/protocol.rs

use anchor_lang::prelude::*;

use crate::instructions::MAX_PAYLOAD_SIZE;

/*
    This file defines serializable data structures intended for off-chain communication.
    The on-chain program does not interpret the content of the `payload` in the `dispatch`
    instructions. It treats it as an opaque byte array (`Vec<u8>`).

    This design pattern turns the Solana blockchain into a secure, decentralized, and
    auditable message broker. Off-chain components (like the `w3b2-connector`) are
    responsible for serializing these structs into the `payload` and deserializing them
    from the corresponding on-chain events. This keeps the on-chain logic minimal and
    gas-efficient while allowing for arbitrarily complex off-chain protocols.
*/

/// Defines the expected communication flow for an off-chain service after
/// receiving a command via a `dispatch` instruction.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandMode {
    /// The off-chain service is expected to process the command and subsequently
    /// initiate a new on-chain transaction (e.g., `admin_dispatch_command`) to
    /// send a response. This creates a two-step, verifiable interaction.
    RequestResponse = 0,
    /// The on-chain command is the final step in the sequence. The off-chain service
    /// executes the requested action, but no on-chain response is expected.
    OneWay = 1,
}

/// Defines a network endpoint for an off-chain service. This allows one party to
/// inform another where to connect for direct, off-chain communication, using the
/// blockchain as the secure introduction mechanism.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// An IPv4 address and a port number for direct socket connections.
    IpV4([u8; 4], u16),
    /// An IPv6 address and a port number for direct socket connections.
    IpV6([u8; 16], u16),
    /// A fully qualified URL string for higher-level protocols (e.g., HTTPS, WSS).
    /// The string is length-prefixed for reliable Borsh serialization.
    Url(String),
}

impl Destination {
    /// Calculates the serialized size of the enum variant.
    pub fn size(&self) -> usize {
        1 + match self {
            // 1 byte for the enum variant tag
            Destination::IpV4(_, _) => 4 + 2, // 4 bytes for IP, 2 bytes for port
            Destination::IpV6(_, _) => 16 + 2, // 16 bytes for IP, 2 bytes for port
            Destination::Url(s) => 4 + s.len(), // 4 bytes for string length prefix + string bytes
        }
    }
}

/// A structured message for initiating a secure, stateful off-chain communication session.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct CommandConfig {
    /// A unique identifier for the off-chain session.
    pub session_id: u64,
    /// A variable-length byte array containing the encrypted session key.
    pub encrypted_session_key: Vec<u8>,
    /// The network endpoint where the initiator expects the recipient to connect.
    pub destination: Destination,
    /// A flexible, general-purpose byte array for any additional metadata.
    pub meta: Vec<u8>,
}

/// An error type for the CommandConfig constructor, used for client-side validation.
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Returned when the total serialized size of the config exceeds `MAX_PAYLOAD_SIZE`.
    PayloadTooLarge {
        calculated_size: usize,
        max_size: usize,
    },
}

impl CommandConfig {
    /// Calculates the total size of the struct when serialized with Borsh.
    fn calculate_size(&self) -> usize {
        // Size of session_id (u64)
        8 +
        // Size of encrypted_session_key (4 bytes for length + content)
        (4 + self.encrypted_session_key.len()) +
        // Size of destination enum (1 byte for tag + content)
        self.destination.size() +
        // Size of meta (4 bytes for length + content)
        (4 + self.meta.len())
    }

    /// Constructs a new `CommandConfig`, validating the total serialized payload size.
    /// This provides a crucial client-side check to prevent sending transactions
    /// that are guaranteed to fail on-chain due to size limits.
    pub fn new(
        session_id: u64,
        encrypted_session_key: Vec<u8>,
        destination: Destination,
        meta: Vec<u8>,
    ) -> std::result::Result<Self, ConfigError> {
        let config = Self {
            session_id,
            encrypted_session_key,
            destination,
            meta,
        };

        let calculated_size = config.calculate_size();

        if calculated_size > MAX_PAYLOAD_SIZE {
            return Err(ConfigError::PayloadTooLarge {
                calculated_size,
                max_size: MAX_PAYLOAD_SIZE,
            });
        }

        Ok(config)
    }
}
