use anchor_lang::prelude::*;

#[error_code]
pub enum BridgeError {
    #[msg("Admin is not authorized to approve this request")]
    Unauthorized,
    #[msg("PDA already registered for this owner")]
    AlreadyRegistered,
    #[msg("Payload too large")]
    PayloadTooLarge,
    #[msg("Funding request has already been processed")]
    RequestAlreadyProcessed,
    #[msg("Insufficient funds for this request")]
    InsufficientFunds,
    #[msg("Account is inactive")]
    InactiveAccount,
    #[msg("Invalid account owner")]
    InvalidAccountOwner,
    #[msg("Invalid account type")]
    InvalidAccountType,
    #[msg("Recipient account is not active")]
    RecipientAccountInactive,
}
