use super::*;

/*
    This file defines the core data structures used for communication within the W3B2 protocol.
    These structs are primarily for off-chain use by the client (e.g., a TypeScript frontend)
    and the server (`w3b2-connector`).

    The on-chain program (`dispatch_command` instruction) does not interpret these structs directly.
    Instead, it treats them as an opaque byte array (`Vec<u8>`) in the `payload`. The client
    serializes a struct (like `CommandConfig`) into this byte array, and the off-chain server
    deserializes it upon receiving the `CommandEvent`.

    This approach keeps the on-chain logic minimal and gas-efficient, acting as a secure message
    broker, while allowing for complex and flexible off-chain communication protocols.
*/

/// `CommandMode` defines the expected behavior for an off-chain service after receiving a command.
/// It is passed within the `dispatch_command` instruction and included in the `CommandEvent`.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandMode {
    /// Indicates a two-step operation. The off-chain service is expected to process the request
    /// and then send a response back to the blockchain, typically via another `dispatch_command`.
    RequestResponse = 0,
    /// Indicates a one-way "fire-and-forget" operation. The off-chain service executes the
    /// action, but no on-chain response is expected.
    OneWay = 1,
}

// NOTE: `CommandRecord` struct has been removed.
// It was redundant because the `CommandEvent` emitted by the `dispatch_command` instruction
// serves the exact same purpose: to log the command details (sender, target, payload, etc.)
// on the blockchain immutably. Relying on the event simplifies the protocol and avoids
// storing duplicate information.

/// `Destination` specifies the network endpoint for the off-chain service.
/// This is a crucial part of `CommandConfig` when establishing a direct, secure connection.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// An IPv4 address and a port number.
    IpV4([u8; 4], u16),
    /// An IPv6 address and a port number.
    IpV6([u8; 16], u16),
    /// A fully qualified URL string (e.g., "https://api.example.com").
    /// The string is prefixed with its length for Borsh serialization.
    Url(String),
}

/// `CommandConfig` is the primary structure for initiating a secure off-chain session.
/// A client creates an instance of this struct, serializes it into Borsh format,
/// and sends it as the `payload` of a `dispatch_command` instruction.
///
/// This is the "agenda" for the meeting, as described in the protocol flow.
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct CommandConfig {
    /// A unique identifier for the communication session, typically a nonce or a timestamp-based u64.
    /// This ID is used in subsequent `log_action` calls to link all related off-chain activities
    /// to this initial session request.
    pub session_id: u64,

    /// The encrypted AES-256 session key, which will be used for symmetric encryption
    /// of the actual data transferred over the direct HTTP channel.
    ///
    /// The encryption process (hybrid encryption) is as follows:
    /// 1. A new, ephemeral X25519 keypair is generated.
    /// 2. A shared secret is derived using the ephemeral private key and the recipient's public `communication_pubkey`.
    /// 3. The shared secret is used to encrypt the 32-byte AES-256 session key.
    /// 4. The final 80-byte block consists of: [ephemeral public key (32) | ciphertext (32) | AEAD tag (16)].
    pub encrypted_session_key: [u8; 80],

    /// The network address where the client expects the off-chain service to connect.
    pub destination: Destination,

    /// An optional, flexible field for any additional application-specific data.
    /// This could include things like API version, request type, or other metadata.
    pub meta: Vec<u8>,
}

/// `FundingStatus` represents the state of a `FundingRequest` PDA.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum FundingStatus {
    /// The initial state of a request. The admin has not yet acted upon it.
    Pending = 0,
    /// The admin has approved the request and transferred the funds.
    Approved = 1,
    /// The admin has rejected the request. (Note: currently, the protocol only handles approval).
    Rejected = 2,
}
