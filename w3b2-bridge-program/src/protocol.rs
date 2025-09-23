use anchor_lang::prelude::*;

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

/// A structured message for initiating a secure, stateful off-chain communication session.
///
/// It is typically serialized and passed in the `payload` of a `dispatch_command`.
/// It serves as the "handshake" to establish a direct, encrypted channel.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct CommandConfig {
    /// A unique identifier for the off-chain session. All subsequent on-chain
    /// (`log_action`) and off-chain activities related to this interaction should be
    /// tagged with this ID for correlation and auditing.
    pub session_id: u64,

    /// A variable-length byte array containing the encrypted session key.
    /// The method of encryption and the format of this field are NOT defined by the
    /// on-chain protocol; they should be agreed upon by the off-chain participants.
    ///
    /// It is EXPECTED that this payload is asymmetrically encrypted using a key
    /// agreement scheme (like X25519 or ECDH) with the recipient's on-chain
    /// `communication_pubkey`. This enables a secure key exchange, establishing
    /// an encrypted channel for direct off-chain communication.
    // CHANGED: from `[u8; 80]` to `Vec<u8>` for flexibility.
    pub encrypted_session_key: Vec<u8>,

    /// The network endpoint where the initiator of the command expects the
    /// recipient to connect for the off-chain part of the session.
    pub destination: Destination,

    /// A flexible, general-purpose byte array for any additional metadata
    /// required by the specific off-chain protocol. This could include protocol
    /// versioning, initial commands, or other setup data.
    pub meta: Vec<u8>,
}
