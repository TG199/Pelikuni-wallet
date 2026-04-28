use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};
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

    pub async fn find_all(&self) -> Result<Vec<Wallet>, sqlx::Error> {
        sqlx::query_as!(
            Wallet,
            r#"
            SELECT id, user_id, balance, version, created_at, updated_at
            FROM wallets
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Updates balance using optimistic concurrency control.
    pub async fn update_balance(
        &self,
        id: Uuid,
        new_balance: Decimal,
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

    /// Atomically transfers funds between two wallets within a single
    /// database transaction. Either both updates succeed and commit,
    /// or the transaction rolls back and neither wallet is modified.
    pub async fn transfer(
        &self,
        from_id: Uuid,
        to_id: Uuid,
        amount: Decimal,
    ) -> Result<(Wallet, Wallet), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Lock both wallets in a consistent order to prevent deadlocks.
        // Two concurrent transfers A→B and B→A would deadlock if each
        // locked its own source first.
        let (lock_first, lock_second) = if from_id < to_id {
            (from_id, to_id)
        } else {
            (to_id, from_id)
        };

        sqlx::query!(
            "SELECT id FROM wallets WHERE id = $1 OR id = $2 ORDER BY id FOR UPDATE",
            lock_first,
            lock_second
        )
        .fetch_all(&mut *tx)
        .await?;

        // Read current state of both wallets
        let from_wallet = Self::find_by_id_in_tx(&mut tx, from_id).await?;
        let to_wallet = Self::find_by_id_in_tx(&mut tx, to_id).await?;

        if from_wallet.balance < amount {
            // Roll back by dropping tx — no explicit rollback needed,
            return Err(sqlx::Error::Protocol("Insufficient balance".into()));
        }

        let updated_from = Self::update_balance_in_tx(
            &mut tx,
            from_id,
            from_wallet.balance - amount,
            from_wallet.version,
        )
        .await?;

        let updated_to = Self::update_balance_in_tx(
            &mut tx,
            to_id,
            to_wallet.balance + amount,
            to_wallet.version,
        )
        .await?;

        tx.commit().await?;

        Ok((updated_from, updated_to))
    }

    async fn find_by_id_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
    ) -> Result<Wallet, sqlx::Error> {
        sqlx::query_as!(
            Wallet,
            r#"
            SELECT id, user_id, balance, version, created_at, updated_at
            FROM wallets
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&mut **tx)
        .await
    }

    async fn update_balance_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
        new_balance: Decimal,
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
        .fetch_one(&mut **tx)
        .await
    }
}
