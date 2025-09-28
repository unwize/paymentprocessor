use crate::errors::KrakenError;

/// Running stats for a Client's account.
/// Does not store individual transactions, just the overall state of the account.
pub struct ClientAccount {
    pub available: f64,
    pub held: f64,
    pub locked: bool,
}

impl ClientAccount {
    pub fn total(&self) -> f64 {
        self.available + self.held
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
            _ => Err(KrakenError::Enum(String::from("Invalid String for TransactionType")))
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
            _ => Err(KrakenError::Enum(String::from("Invalid String for TransactionType")))
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