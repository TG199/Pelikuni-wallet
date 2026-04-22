use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::wallet::Wallet;

pub struct WalletRepository {
    pool: PgPool,
}

impl WalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, wallet: &Wallet) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as!(
            Wallet,
            r#"
            INSERT INTO wallets (id, user_id, balance, version, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, balance, version, created_at, updated_at
            "#,
            wallet.id,
            wallet.user_id,
            wallet.balance,
            wallet.version,
            wallet.created_at,
            wallet.updated_at
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as!(
            Wallet,
            r#"
            SELECT id, user_id, balance, version, created_at, updated_at
            FROM wallets
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn find_by_user(&self, user_id: &str) -> Result<Vec<Wallet>, sqlx::Error> {
        sqlx::query_as!(
            Wallet,
            r#"
            SELECT id, user_id, balance, version, created_at, updated_at
            FROM wallets
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn update_balance(
        &self,
        id: Uuid,
        new_balance: rust_decimal::Decimal,
        expected_version: i64,
    ) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as!(
            Wallet,
            r#"
            UPDATE wallets
            SET balance = $1, version = version + 1, updated_at = NOW()
            WHERE id = $2 AND version = $3
            RETURNING id, user_id, balance, version, created_at, updated_at
            "#,
            new_balance,
            id,
            expected_version
        )
        .fetch_one(&self.pool)
        .await
    }
}
