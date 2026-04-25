use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    WalletCreated,
    WalletFunded,
    TransferCompleted,
    TransferFailed,
}

pub trait Event {
    fn event_type(&self) -> EventType;
    fn timestamp(&self) -> DateTime<Utc>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WalletEvent {
    WalletCreated {
        wallet_id: Uuid,
        user_id: String,
        transaction_id: Uuid, // fixed: was transanction_id
        initial_balance: Decimal,
        timestamp: DateTime<Utc>,
    },
    WalletFunded {
        wallet_id: Uuid,
        user_id: String,
        transaction_id: Uuid,
        amount: Decimal,
        new_balance: Decimal,
        timestamp: DateTime<Utc>,
    },
    TransferCompleted {
        from_wallet_id: Uuid,
        to_wallet_id: Uuid,
        from_user_id: String,
        to_user_id: String,
        amount: Decimal,
        from_transaction_id: Uuid,
        to_transaction_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    TransferFailed {
        from_wallet_id: Uuid,
        to_wallet_id: Uuid,
        from_user_id: String,
        amount: Decimal,
        reason: String,
        timestamp: DateTime<Utc>,
    },
}

impl WalletEvent {
    pub fn event_type(&self) -> EventType {
        match self {
            Self::WalletCreated { .. } => EventType::WalletCreated,
            Self::WalletFunded { .. } => EventType::WalletFunded,
            Self::TransferCompleted { .. } => EventType::TransferCompleted,
            Self::TransferFailed { .. } => EventType::TransferFailed,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::WalletCreated { timestamp, .. }
            | Self::WalletFunded { timestamp, .. }
            | Self::TransferCompleted { timestamp, .. }
            | Self::TransferFailed { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the primary wallet ID as the Kafka partition key.
    /// Ensures all events for a wallet land on the same partition,
    /// preserving per-wallet ordering.
    pub fn wallet_key(&self) -> String {
        match self {
            Self::WalletCreated { wallet_id, .. } => wallet_id.to_string(),
            Self::WalletFunded { wallet_id, .. } => wallet_id.to_string(),
            Self::TransferCompleted { from_wallet_id, .. } => from_wallet_id.to_string(),
            Self::TransferFailed { from_wallet_id, .. } => from_wallet_id.to_string(),
        }
    }
}
