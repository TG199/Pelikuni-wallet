use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::{App, Json, Path};
use uuid::Uuid;

use crate::domain::{CreateWalletRequest, Wallet, WalletRepository, WalletResponse};

pub async fn create_wallet(
    repo: web::Data<WalletRepository>,
    payload: Json<CreateWalletRequest>,
) -> Result<(StatusCode, Json<WalletResponse>), (StatusCode, String)> {
    let wallet = Wallet::new(payload.user_id);

    let created_wallet = repo
        .create(&wallet)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(created_wallet.into())))
}

pub async fn get_wallet(
    repo: web::Data<WalletRepository>,
    id: Path<Uuid>,
) -> Result<Json<WalletResponse>, (StatusCode, String)> {
    let wallet = repo.find_by_id(id).await.map_err(|e| match e {
        sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Wallet not found".to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    })?;

    Ok(Json(wallet.into()))
}

pub async fn list_user_wallets(
    repo: web::Data<WalletRepository>,
    user_id: Path<String>,
) -> Result<Json<Vec<WalletResponse>>, (StatusCode, String)> {
    let wallets = repo
        .find_by_user(&user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<WalletResponse> = wallets.into_iter().map(Into::into).collect();

    Ok(Json(responses))
}
