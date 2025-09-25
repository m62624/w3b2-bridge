use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use borsh::BorshDeserialize;

// Import all the on-chain event structs and give them a clear alias.
use w3b2_bridge_program::events as OnChainEvent;

/// A connector-side enum that wraps all possible on-chain events.
/// This provides a single, unified type for the dispatcher to work with.
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    AdminProfileRegistered(OnChainEvent::AdminProfileRegistered),
    AdminCommKeyUpdated(OnChainEvent::AdminCommKeyUpdated),
    AdminPricesUpdated(OnChainEvent::AdminPricesUpdated),
    AdminFundsWithdrawn(OnChainEvent::AdminFundsWithdrawn),
    AdminProfileClosed(OnChainEvent::AdminProfileClosed),
    AdminCommandDispatched(OnChainEvent::AdminCommandDispatched),
    UserProfileCreated(OnChainEvent::UserProfileCreated),
    UserCommKeyUpdated(OnChainEvent::UserCommKeyUpdated),
    UserFundsDeposited(OnChainEvent::UserFundsDeposited),
    UserFundsWithdrawn(OnChainEvent::UserFundsWithdrawn),
    UserProfileClosed(OnChainEvent::UserProfileClosed),
    UserCommandDispatched(OnChainEvent::UserCommandDispatched),
    OffChainActionLogged(OnChainEvent::OffChainActionLogged),
    Unknown,
}

/// Parses the raw event data from a log message.
/// It identifies the event type by its 8-byte discriminator and deserializes
/// the rest of the data into the corresponding struct.
pub fn parse_event_data(data: &[u8]) -> Result<BridgeEvent> {
    if data.len() < 8 {
        return Ok(BridgeEvent::Unknown);
    }

    let discriminator = &data[0..8];
    let event_data = &data[8..];

    // This macro simplifies calculating the discriminator for each event.
    macro_rules! get_disc {
        ($name:literal) => {
            anchor_lang::solana_program::hash::hash(format!("event:{}", $name).as_bytes())
                .to_bytes()[0..8]
                .to_vec()
        };
    }

    // Compare the discriminator from the log with the known discriminators.
    if discriminator == get_disc!("AdminProfileRegistered").as_slice() {
        let event = OnChainEvent::AdminProfileRegistered::try_from_slice(event_data)?;
        Ok(BridgeEvent::AdminProfileRegistered(event))
    } else if discriminator == get_disc!("AdminCommKeyUpdated").as_slice() {
        let event = OnChainEvent::AdminCommKeyUpdated::try_from_slice(event_data)?;
        Ok(BridgeEvent::AdminCommKeyUpdated(event))
    } else if discriminator == get_disc!("AdminPricesUpdated").as_slice() {
        let event = OnChainEvent::AdminPricesUpdated::try_from_slice(event_data)?;
        Ok(BridgeEvent::AdminPricesUpdated(event))
    } else if discriminator == get_disc!("AdminFundsWithdrawn").as_slice() {
        let event = OnChainEvent::AdminFundsWithdrawn::try_from_slice(event_data)?;
        Ok(BridgeEvent::AdminFundsWithdrawn(event))
    } else if discriminator == get_disc!("AdminProfileClosed").as_slice() {
        let event = OnChainEvent::AdminProfileClosed::try_from_slice(event_data)?;
        Ok(BridgeEvent::AdminProfileClosed(event))
    } else if discriminator == get_disc!("AdminCommandDispatched").as_slice() {
        let event = OnChainEvent::AdminCommandDispatched::try_from_slice(event_data)?;
        Ok(BridgeEvent::AdminCommandDispatched(event))
    } else if discriminator == get_disc!("UserProfileCreated").as_slice() {
        let event = OnChainEvent::UserProfileCreated::try_from_slice(event_data)?;
        Ok(BridgeEvent::UserProfileCreated(event))
    } else if discriminator == get_disc!("UserCommKeyUpdated").as_slice() {
        let event = OnChainEvent::UserCommKeyUpdated::try_from_slice(event_data)?;
        Ok(BridgeEvent::UserCommKeyUpdated(event))
    } else if discriminator == get_disc!("UserFundsDeposited").as_slice() {
        let event = OnChainEvent::UserFundsDeposited::try_from_slice(event_data)?;
        Ok(BridgeEvent::UserFundsDeposited(event))
    } else if discriminator == get_disc!("UserFundsWithdrawn").as_slice() {
        let event = OnChainEvent::UserFundsWithdrawn::try_from_slice(event_data)?;
        Ok(BridgeEvent::UserFundsWithdrawn(event))
    } else if discriminator == get_disc!("UserProfileClosed").as_slice() {
        let event = OnChainEvent::UserProfileClosed::try_from_slice(event_data)?;
        Ok(BridgeEvent::UserProfileClosed(event))
    } else if discriminator == get_disc!("UserCommandDispatched").as_slice() {
        let event = OnChainEvent::UserCommandDispatched::try_from_slice(event_data)?;
        Ok(BridgeEvent::UserCommandDispatched(event))
    } else if discriminator == get_disc!("OffChainActionLogged").as_slice() {
        let event = OnChainEvent::OffChainActionLogged::try_from_slice(event_data)?;
        Ok(BridgeEvent::OffChainActionLogged(event))
    } else {
        Ok(BridgeEvent::Unknown)
    }
}

/// Attempts to extract a base64 payload from a log line and parse it into an event.
/// This function looks for the "Program data: " prefix added by `emit!`.
pub fn try_parse_log(log: &str) -> Result<BridgeEvent> {
    if let Some(data_str) = log.strip_prefix("Program data: ") {
        if let Ok(bytes) = BASE64.decode(data_str.trim()) {
            if let Ok(event) = parse_event_data(&bytes) {
                // Only return successfully parsed, known events.
                if !matches!(event, BridgeEvent::Unknown) {
                    return Ok(event);
                }
            }
        }
    }
    Ok(BridgeEvent::Unknown)
}
