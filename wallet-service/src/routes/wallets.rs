use actix_web::error::{
    ErrorBadRequest, ErrorConflict, ErrorInternalServerError, ErrorNotFound,
    ErrorUnprocessableEntity,
};
use actix_web::web;
use actix_web::Error;
use actix_web::HttpResponse;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::Deserialize;
use shared::WalletEvent;
use uuid::Uuid;

use crate::domain::{CreateWalletRequest, Wallet, WalletRepository, WalletResponse};
use crate::kafka::KafkaProducer;

pub async fn create_wallet(
    repo: web::Data<WalletRepository>,
    producer: web::Data<KafkaProducer>,
    payload: web::Json<CreateWalletRequest>,
) -> Result<HttpResponse, Error> {
    let wallet = Wallet::new(payload.user_id.clone());

    let created = repo
        .create(&wallet)
        .await
        .map_err(|e| ErrorInternalServerError(e.to_string()))?;

    // Publish event — fire and forget, don't fail the request if Kafka is down
    let event = WalletEvent::WalletCreated {
        wallet_id: created.id,
        user_id: created.user_id.clone(),
        transaction_id: Uuid::new_v4(),
        initial_balance: created.balance,
        timestamp: Utc::now(),
    };

    if let Err(e) = producer.publish(&event).await {
        tracing::warn!(error = %e, "Failed to publish WalletCreated event");
    }

    Ok(HttpResponse::Created().json(WalletResponse::from(created)))
}

pub async fn get_wallet(
    repo: web::Data<WalletRepository>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, Error> {
    let wallet =
        repo.find_by_id(id.into_inner())
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => ErrorNotFound("Wallet not found"),
                _ => ErrorInternalServerError(e.to_string()),
            })?;

    Ok(HttpResponse::Ok().json(WalletResponse::from(wallet)))
}

pub async fn list_user_wallets(
    repo: web::Data<WalletRepository>,
    user_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let wallets = repo
        .find_by_user(&user_id.into_inner())
        .await
        .map_err(|e| ErrorInternalServerError(e.to_string()))?;

    let responses: Vec<WalletResponse> = wallets.into_iter().map(Into::into).collect();
    Ok(HttpResponse::Ok().json(responses))
}

#[derive(Debug, Deserialize)]
pub struct FundWalletRequest {
    pub amount: Decimal,
}

pub async fn fund_wallet(
    repo: web::Data<WalletRepository>,
    producer: web::Data<KafkaProducer>,
    id: web::Path<Uuid>,
    payload: web::Json<FundWalletRequest>,
) -> Result<HttpResponse, Error> {
    if payload.amount <= Decimal::ZERO {
        return Err(ErrorBadRequest("Amount must be greater than zero"));
    }

    let wallet_id = id.into_inner();

    let wallet = repo.find_by_id(wallet_id).await.map_err(|e| match e {
        sqlx::Error::RowNotFound => ErrorNotFound("Wallet not found"),
        _ => ErrorInternalServerError(e.to_string()),
    })?;

    let new_balance = wallet.balance + payload.amount;

    let updated = repo
        .update_balance(wallet.id, new_balance, wallet.version)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                ErrorConflict("Concurrent modification detected, please retry")
            }
            _ => ErrorInternalServerError(e.to_string()),
        })?;

    let transaction_id = Uuid::new_v4();
    let event = WalletEvent::WalletFunded {
        wallet_id: updated.id,
        user_id: updated.user_id.clone(),
        transaction_id,
        amount: payload.amount,
        new_balance: updated.balance,
        timestamp: Utc::now(),
    };

    if let Err(e) = producer.publish(&event).await {
        tracing::warn!(error = %e, "Failed to publish WalletFunded event");
    }

    Ok(HttpResponse::Ok().json(WalletResponse::from(updated)))
}

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub to_wallet_id: Uuid,
    pub amount: Decimal,
}

pub async fn transfer(
    repo: web::Data<WalletRepository>,
    producer: web::Data<KafkaProducer>,
    from_id: web::Path<Uuid>,
    payload: web::Json<TransferRequest>,
) -> Result<HttpResponse, Error> {
    if payload.amount <= Decimal::ZERO {
        return Err(ErrorBadRequest("Amount must be greater than zero"));
    }

    let from_wallet = repo
        .find_by_id(from_id.into_inner())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ErrorNotFound("Source wallet not found"),
            _ => ErrorInternalServerError(e.to_string()),
        })?;

    if from_wallet.balance < payload.amount {
        return Err(ErrorUnprocessableEntity("Insufficient balance"));
    }

    let to_wallet = repo
        .find_by_id(payload.to_wallet_id)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ErrorNotFound("Destination wallet not found"),
            _ => ErrorInternalServerError(e.to_string()),
        })?;

    // Debit source
    let new_from_balance = from_wallet.balance - payload.amount;
    repo.update_balance(from_wallet.id, new_from_balance, from_wallet.version)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                ErrorConflict("Concurrent modification on source wallet, please retry")
            }
            _ => ErrorInternalServerError(e.to_string()),
        })?;

    // Credit destination
    let new_to_balance = to_wallet.balance + payload.amount;
    let updated_to = repo
        .update_balance(to_wallet.id, new_to_balance, to_wallet.version)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                ErrorConflict("Concurrent modification on destination wallet, please retry")
            }
            _ => ErrorInternalServerError(e.to_string()),
        })?;

    let from_transaction_id = Uuid::new_v4();
    let to_transaction_id = Uuid::new_v4();

    let event = WalletEvent::TransferCompleted {
        from_wallet_id: from_wallet.id,
        to_wallet_id: to_wallet.id,
        from_user_id: from_wallet.user_id.clone(),
        to_user_id: to_wallet.user_id.clone(),
        amount: payload.amount,
        from_transaction_id,
        to_transaction_id,
        timestamp: Utc::now(),
    };

    if let Err(e) = producer.publish(&event).await {
        tracing::warn!(error = %e, "Failed to publish TransferCompleted event");
    }

    Ok(HttpResponse::Ok().json(WalletResponse::from(updated_to)))
}
