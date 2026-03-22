use actix_web::{
    web::{Data, Json, Path},
    HttpResponse, Result,
};
use uuid::Uuid;

use crate::domain::{CreateWalletRequest, Wallet, WalletRepository, WalletResponse};

pub async fn create_wallet(
    repo: Data<WalletRepository>,
    payload: Json<CreateWalletRequest>,
) -> Result<HttpResponse> {
    let payload = payload.into_inner(); // ✅ extract
    let wallet = Wallet::new(payload.user_id);

    let created_wallet = repo
        .create(&wallet)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Created().json(WalletResponse::from(created_wallet)))
}

pub async fn get_wallet(repo: Data<WalletRepository>, id: Path<Uuid>) -> Result<HttpResponse> {
    let id = id.into_inner(); // ✅ extract

    let wallet = repo.find_by_id(id).await.map_err(|e| match e {
        sqlx::Error::RowNotFound => actix_web::error::ErrorNotFound("Wallet not found"),
        _ => actix_web::error::ErrorInternalServerError(e),
    })?;

    Ok(HttpResponse::Ok().json(WalletResponse::from(wallet)))
}

pub async fn list_user_wallets(
    repo: Data<WalletRepository>,
    user_id: Path<String>,
) -> Result<HttpResponse> {
    let user_id = user_id.into_inner(); // ✅ extract

    let wallets = repo
        .find_by_user(&user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let responses: Vec<WalletResponse> = wallets.into_iter().map(Into::into).collect();

    Ok(HttpResponse::Ok().json(responses))
}
