use crate::errors::KrakenError;
use crate::errors::KrakenError::{AccountLocked, DisputeStateError, InsufficientFunds};
use std::collections::HashMap;

/// Running stats for a Client's account.
/// Does not store individual transactions, just the overall state of the account.
pub struct ClientAccount {
    pub available: f64,
    pub held: f64,
    pub locked: bool,
    pub history: HashMap<u32, Transaction>, // A map of TX to Transaction. Only Deposits and Withdrawals are stored.
}

impl ClientAccount {
    pub fn total(&self) -> f64 {
        self.available + self.held
    }

    /// Move a Transaction object into the `history` field and then apply logic to the account.
    /// Invalid transactions are dropped.
    pub fn apply_transaction(&mut self, transaction: Transaction) -> Result<(), KrakenError> {
        match &transaction.kind {
            TransactionType::Deposit => {
                if self.locked {
                    return Err(AccountLocked(transaction.client));
                }

                self.available += transaction.amount;

                self.history.insert(transaction.tx, transaction); // Move to history
                Ok(())
            }
            TransactionType::Withdrawal => {
                if self.locked {
                    return Err(AccountLocked(transaction.client));
                }

                if self.available < transaction.amount {
                    return Err(InsufficientFunds(transaction.client));
                }

                self.available -= transaction.amount;

                self.history.insert(transaction.tx, transaction); // Move to history
                Ok(())
            }
            TransactionType::Dispute => {
                // Allow locked accounts to still dispute
                if let Some(transaction) = self.history.get_mut(&transaction.tx) {
                    if transaction.state.is_some() {
                        return Err(DisputeStateError(String::from(
                            "Transaction already disputed",
                        )));
                    }

                    transaction.state = Some(TransactionType::Dispute);
                    self.available -= transaction.amount;
                    self.held += transaction.amount;
                    Ok(())
                }
            }
            TransactionType::Resolve => {}
            TransactionType::Chargeback => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionType {
    Deposit = 0,
    Withdrawal = 1,
    Dispute = 2,
    Resolve = 3,
    Chargeback = 4,
}

impl TryFrom<String> for TransactionType {
    type Error = KrakenError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "deposit" => Ok(TransactionType::Deposit),
            "withdrawal" => Ok(TransactionType::Withdrawal),
            "dispute" => Ok(TransactionType::Dispute),
            "resolve" => Ok(TransactionType::Resolve),
            "chargeback" => Ok(TransactionType::Chargeback),
            _ => Err(KrakenError::Enum(String::from(
                "Invalid String for TransactionType",
            ))),
        }
    }
}

impl TryFrom<&str> for TransactionType {
    type Error = KrakenError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "deposit" => Ok(TransactionType::Deposit),
            "withdrawal" => Ok(TransactionType::Withdrawal),
            "dispute" => Ok(TransactionType::Dispute),
            "resolve" => Ok(TransactionType::Resolve),
            "chargeback" => Ok(TransactionType::Chargeback),
            _ => Err(KrakenError::Enum(String::from(
                "Invalid String for TransactionType",
            ))),
        }
    }
}

#[derive(Debug)]
pub struct Transaction {
    pub kind: TransactionType,
    pub client: u32,
    pub amount: f64,
    pub tx: u32,
    pub state: Option<TransactionType>,
}
