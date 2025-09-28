use thiserror::Error;

#[derive(Error, Debug)]
pub enum KrakenError {
    #[error("IO Error")]
    IO,

    #[error("Invalid enum value error: {0}")]
    Enum(String),

    #[error("Dispute Chronology Error: Base: {0}, Attempt: {1}")]
    DisputeChronoError(u32, u32),

    #[error("Dispute State Error: {0}")]
    DisputeStateError(String),

    #[error("No Such Transaction Error: {0}")]
    NoSuchTransactionError(String),

    #[error("Account is locked: {0}")]
    AccountLocked(u32),

    #[error("Insufficient Funds for account: {0}")]
    InsufficientFunds(u32),

    #[error("Error")]
    Error,
}
