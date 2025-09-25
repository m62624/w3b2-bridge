# W3B2 Bridge Program

An on-chain program for the W3B2 Bridge protocol, enabling secure, standardized interaction between Web2 services and the Solana blockchain.

## Philosophy & Core Concept

The W3B2 Bridge protocol is designed to provide a familiar Web2-like user experience (UX) for blockchain interactions. It abstracts away asset management for everyday operations while retaining the security and transparency of the Solana network.

The architecture is built on the **"Isolated Service Wallets (`ChainCard`)"** model. For each service a user interacts with, a dedicated, isolated `ChainCard` (a standard Solana Keypair) is used to sign all transactions for that specific service. This leads to:

  * **Asset Isolation:** Operations with one service do not affect or clutter the transaction history of the user's other wallets.
  * **Enhanced Security:** A potential compromise of one service's `ChainCard` does not impact the user's other assets.
  * **Simplified UX:** The user interacts with a standard application interface, while an off-chain component (`w3b2-connector`) manages the `ChainCard` under the hood.

## Core On-Chain Entities

The program uses two primary Program Derived Addresses (PDAs) to manage state:

  * **`AdminProfile` PDA**

      * **Represents:** A Web2 service provider (an "Admin").
      * **Stores:** The admin's `authority` key (`ChainCard`), a `communication_pubkey` for off-chain encryption, a dynamic `prices` list for its API, and its earned `balance`.
      * **PDA Seeds:** `[b"admin", authority.key().as_ref()]`

  * **`UserProfile` PDA**

      * **Represents:** A user's relationship with and financial deposit for a *specific* Admin service.
      * **Stores:** The user's `authority` key (`ChainCard`), a `communication_pubkey`, the `admin_authority_on_creation` it's linked to, and the user's `deposit_balance`.
      * **PDA Seeds:** `[b"user", authority.key().as_ref(), admin_profile.key().as_ref()]`

## Instruction Interface

All state-changing instructions require a signature from the appropriate `ChainCard` (`authority`).

### Admin Instructions

| Instruction              | Signer            | Arguments                      | Description                                                                 |
| ------------------------ | ----------------- | ------------------------------ | --------------------------------------------------------------------------- |
| `admin_register_profile` | Admin `ChainCard` | `communication_pubkey: Pubkey` | Creates the `AdminProfile` PDA for a new service.                           |
| `admin_update_comm_key`  | Admin `ChainCard` | `new_key: Pubkey`              | Updates the admin's off-chain communication public key.                     |
| `admin_update_prices`    | Admin `ChainCard` | `new_prices: Vec<(u16, u64)>`  | Updates the service price list. The PDA is reallocated to fit the new size. |
| `admin_withdraw`         | Admin `ChainCard` | `amount: u64`                  | Withdraws earned funds from the `AdminProfile`'s balance to a destination.  |
| `admin_close_profile`    | Admin `ChainCard` | -                              | Closes the `AdminProfile` and refunds the rent to the admin's `authority`.  |

### User Instructions

| Instruction            | Signer           | Arguments                                              | Description                                                                               |
| ---------------------- | ---------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| `user_create_profile`  | User `ChainCard` | `target_admin: Pubkey`, `communication_pubkey: Pubkey` | Creates a `UserProfile` PDA, linking the user to a specific admin service.                |
| `user_update_comm_key` | User `ChainCard` | `new_key: Pubkey`                                      | Updates the user's off-chain communication public key for a specific service profile.     |
| `user_deposit`         | User `ChainCard` | `amount: u64`                                          | Deposits lamports into the `UserProfile` PDA to fund future command calls.                |
| `user_withdraw`        | User `ChainCard` | `amount: u64`                                          | Withdraws unspent funds from the `UserProfile`'s deposit balance.                         |
| `user_close_profile`   | User `ChainCard` | -                                                      | Closes the `UserProfile` and refunds all remaining lamports (deposit + rent) to the user. |

### Operational Instructions

These instructions facilitate the primary bidirectional communication flow.

| Instruction              | Signer            | Arguments                             | Description                                                                                                                     |
| ------------------------ | ----------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `user_dispatch_command`  | User `ChainCard`  | `command_id: u16`, `payload: Vec<u8>` | A user calls a service's API. If the command has a price, funds are transferred from the user's deposit to the admin's balance. |
| `admin_dispatch_command` | Admin `ChainCard` | `command_id: u16`, `payload: Vec<u8>` | An admin sends a command/notification to a user. This is a non-financial transaction used to emit an event.                     |
| `log_action`             | User or Admin     | `session_id: u64`, `action_code: u16` | A generic instruction to log a significant off-chain action to the blockchain for auditing purposes.                            |

## Off-Chain Communication & Events

The primary mechanism for the on-chain program to communicate with the off-chain world (e.g., the `w3b2-connector`) is through Solana events.

### Event Structure (Protobuf)

All events emitted by the program are formally defined using **Protocol Buffers (Protobuf)**. The schema is located in the `proto/events.proto` file and serves as the **single source of truth** for all event data structures.

This file defines a main `BridgeEvent` message with a `oneof` field, which acts as a wrapper that can contain any of the specific event types (e.g., `AdminProfileRegistered`, `UserCommandDispatched`). This allows off-chain clients to handle all events from a single, strongly-typed source.

### Event Streaming (gRPC)

The `proto/events.proto` file also defines a **gRPC** service:

```protobuf
service BridgeService {
  rpc StreamEvents(Empty) returns (stream BridgeEvent);
}
```

This specifies a real-time streaming API for events. An off-chain component like the `w3b2-connector` can listen for on-chain events, parse them using the Protobuf definitions, and then expose them to other services via this gRPC `StreamEvents` endpoint.

