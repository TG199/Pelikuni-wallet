use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: String,
    pub balance: Decimal,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Wallet {
    pub fn new(user_id: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            balance: Decimal::ZERO,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct WalletResponse {
    pub id: Uuid,
    pub user_id: String,
    pub balance: Decimal,
    pub version: i64,
}

impl From<Wallet> for WalletResponse {
    fn from(wallet: Wallet) -> Self {
        Self {
            id: wallet.id,
            user_id: wallet.user_id,
            balance: wallet.balance,
            version: wallet.version,
        }
    }
}
