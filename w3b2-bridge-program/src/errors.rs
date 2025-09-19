use anchor_lang::prelude::*;

#[error_code]
pub enum BridgeError {
    #[msg("Unauthorized: Signer does not match the authority on the account.")]
    Unauthorized,

    #[msg("Rent-Exempt Violation: This transaction would leave the PDA with a balance below the rent-exempt minimum.")]
    RentExemptViolation,
    
    #[msg("Insufficient Deposit Balance: The user's deposit is not enough to pay for this command.")]
    InsufficientDepositBalance,

    #[msg("Insufficient PDA Balance: The PDA does not have enough lamports to cover the withdrawal amount.")]
    InsufficientPDABalance,

    #[msg("Command Not Found: The requested command_id does not exist in the admin's price list.")]
    CommandNotFound,

    #[msg("Payload Too Large: The provided payload exceeds the maximum allowed size.")]
    PayloadTooLarge,
}