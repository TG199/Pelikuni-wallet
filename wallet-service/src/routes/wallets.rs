use actix_web::error::{ErrorBadRequest, ErrorConflict, ErrorInternalServerError, ErrorNotFound};
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
    let wallet = repo
        .find_by_id(id.into_inner())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ErrorNotFound("Wallet not found"),
            _ => ErrorInternalServerError(e.to_string()),
        })?;

    Ok(HttpResponse::Ok().json(WalletResponse::from(wallet)))
}

pub async fn list_wallets(repo: web::Data<WalletRepository>) -> Result<HttpResponse, Error> {
    let wallets = repo
        .find_all()
        .await
        .map_err(|e| ErrorInternalServerError(e.to_string()))?;

    let responses: Vec<WalletResponse> = wallets.into_iter().map(Into::into).collect();
    Ok(HttpResponse::Ok().json(responses))
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

    let event = WalletEvent::WalletFunded {
        wallet_id: updated.id,
        user_id: updated.user_id.clone(),
        transaction_id: Uuid::new_v4(),
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

    let from_uuid = from_id.into_inner();

    if from_uuid == payload.to_wallet_id {
        return Err(ErrorBadRequest("Cannot transfer to the same wallet"));
    }

    let (updated_from, updated_to) = repo
        .transfer(from_uuid, payload.to_wallet_id, payload.amount)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ErrorNotFound("Wallet not found"),
            sqlx::Error::Protocol(msg) if msg.contains("Insufficient balance") => {
                ErrorBadRequest("Insufficient balance")
            }
            _ => ErrorInternalServerError(e.to_string()),
        })?;

    let event = WalletEvent::TransferCompleted {
        from_wallet_id: updated_from.id,
        to_wallet_id: updated_to.id,
        from_user_id: updated_from.user_id.clone(),
        to_user_id: updated_to.user_id.clone(),
        amount: payload.amount,
        from_transaction_id: Uuid::new_v4(),
        to_transaction_id: Uuid::new_v4(),
        timestamp: Utc::now(),
    };

    if let Err(e) = producer.publish(&event).await {
        tracing::warn!(error = %e, "Failed to publish TransferCompleted event");
    }

    Ok(HttpResponse::Ok().json(WalletResponse::from(updated_to)))
}
