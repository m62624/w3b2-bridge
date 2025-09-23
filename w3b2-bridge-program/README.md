# W3B2 Bridge Program

An on-chain program for the W3B2 Bridge protocol, enabling secure, standardized interaction between Web2 services and the Solana blockchain.

## Philosophy & Core Concept

The W3B2 Bridge protocol is designed to provide a seamless Web2-like user experience (UX) for blockchain interactions. It abstracts away the complexities of asset management for everyday operations while retaining the security, transparency, and decentralization of the Solana network.

The architecture is built on the **"Isolated Service Wallets (`ChainCard`)"** model. For each service a user interacts with, a dedicated, isolated `ChainCard` (a standard Solana Keypair) is used. This wallet signs all transactions for that specific service, leading to:

  * **Asset Isolation:** Operations with one service do not affect or clutter the transaction history of the user's other wallets.
  * **Enhanced Security:** A potential compromise of one service's `ChainCard` does not impact the user's other assets.
  * **Simplified UX:** The user interacts with a familiar application interface, while an off-chain component (`w3b2-connector`) manages the `ChainCard` under the hood.

## Core On-Chain Entities

The program uses two primary Program Derived Addresses (PDAs) to manage state:

  * **`AdminProfile` PDA**

      * **Represents:** A Web2 service provider (an "Admin").
      * **Stores:** The admin's `authority` key, a `communication_pubkey` for off-chain encryption, a dynamic `prices` list for its API, and its earned `balance`.
      * **PDA Seeds:** `[b"admin", authority.key().as_ref()]`

  * **`UserProfile` PDA**

      * **Represents:** A user's relationship with and financial deposit for a *specific* Admin service.
      * **Stores:** The user's `authority` key, a `communication_pubkey`, the `admin_authority_on_creation` it's linked to, and the user's `deposit_balance`.
      * **PDA Seeds:** `[b"user", authority.key().as_ref(), admin_profile.key().as_ref()]`

## Instruction Interface

All state-changing instructions require a signature from the appropriate `ChainCard` (`authority`).

### Admin Instructions

| Instruction              | Signer            | Arguments                      | Description                                                                 |
| ------------------------ | ----------------- | ------------------------------ | --------------------------------------------------------------------------- |
| `admin_register_profile` | Admin `ChainCard` | `communication_pubkey: Pubkey` | Creates the `AdminProfile` PDA for a new service.                           |
| `admin_update_comm_key`  | Admin `ChainCard` | `new_key: Pubkey`              | Updates the admin's off-chain communication public key.                     |
| `admin_update_prices`    | Admin `ChainCard` | `new_prices: Vec<(u64, u64)>`  | Updates the service price list. The PDA is reallocated to fit the new size. |
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
| `user_dispatch_command`  | User `ChainCard`  | `command_id: u64`, `payload: Vec<u8>` | A user calls a service's API. If the command has a price, funds are transferred from the user's deposit to the admin's balance. |
| `admin_dispatch_command` | Admin `ChainCard` | `command_id: u64`, `payload: Vec<u8>` | An admin sends a command/notification to a user. This is a non-financial transaction used to emit an event.                     |
| `log_action`             | User or Admin     | `session_id: u64`, `action_code: u16` | A generic instruction to log a significant off-chain action to the blockchain for auditing purposes.                            |

## Events

The program emits events for every significant action, allowing off-chain services (`w3b2-connector`) to listen and react. Key events include:

  * `AdminProfileRegistered`, `AdminPricesUpdated`, `AdminFundsWithdrawn`
  * `UserProfileCreated`, `FundsDeposited`, `FundsWithdrawn`
  * `UserCommandDispatched`, `AdminCommandDispatched`
  * `HttpActionLogged`

## Security Model

The protocol's security relies on several core Anchor and Solana features:

  * **PDA Validation:** All PDA accounts are verified on every instruction using `seeds` checks to prevent account spoofing.
  * **Authority Checks:** State-changing instructions use `constraint` checks to ensure the `Signer` is the legitimate owner of the on-chain profile.
  * **Relationship Integrity:** Cross-account relationships (e.g., ensuring a `UserProfile` is interacting with the correct `AdminProfile`) are enforced through a combination of `seeds` and `constraints`.
  * **Rent Exemption:** All instructions that debit lamports from a PDA (`withdraw`, `user_dispatch_command`) ensure that the remaining balance does not fall below the rent-exempt minimum, preventing account closure by the Solana runtime.