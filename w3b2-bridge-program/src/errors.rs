use anchor_lang::prelude::*;

#[error_code]
pub enum BridgeError {
    #[msg("Unauthorized: The signer does not have permission to perform this action.")]
    Unauthorized,

    #[msg("PayerInsufficientFunds: The payer does not have enough lamports to create the account and provide the initial balance.")]
    PayerInsufficientFunds,

    #[msg("AdminInsufficientFunds: The admin PDA does not have enough lamports to approve the funding request.")]
    AdminInsufficientFunds,

    #[msg("Payload too large: The provided payload exceeds the 1024-byte limit.")]
    PayloadTooLarge,

    #[msg(
        "Request already processed: This funding request has already been approved or rejected."
    )]
    RequestAlreadyProcessed,

    #[msg("Sender account is inactive: The sender's account is currently deactivated.")]
    SenderInactive,

    #[msg("Recipient account is inactive: The recipient's account is currently deactivated.")]
    RecipientInactive,

    #[msg("Invalid account owner: The account is not owned by the W3B2 bridge program.")]
    InvalidAccountOwner,

    #[msg(
        "Invalid account type: The provided account is not a valid UserAccount or AdminAccount."
    )]
    InvalidAccountType,

    #[msg("Rent-Exempt Violation: This transaction would leave the account with a balance below the rent-exempt minimum.")]
    RentExemptViolation,
}
