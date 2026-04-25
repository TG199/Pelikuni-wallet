use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use reqwest::Url;
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::domain::WalletRepository;
use crate::kafka::KafkaProducer;
use crate::routes::{
    create_wallet, fund_wallet, get_wallet, health_check, home, list_user_wallets, transfer,
};

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();

        // Kafka is optional — service starts without it, events are skipped with a warning
        let kafka_producer = configuration.kafka.as_ref().and_then(|k| {
            match KafkaProducer::new(&k.brokers, &k.topic) {
                Ok(p) => {
                    tracing::info!("Kafka producer initialized on topic '{}'", k.topic);
                    Some(p)
                }
                Err(e) => {
                    tracing::warn!("Kafka unavailable, events will not be published: {}", e);
                    None
                }
            }
        });

        let server = run(
            listener,
            connection_pool,
            configuration.application.url().expect("Invalid host url"),
            configuration.application.hmac_secret,
            kafka_producer,
        )
        .await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

pub struct ApplicationBaseUrl(pub Url);

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    base_url: Url,
    hmac_secret: SecretString,
    kafka_producer: Option<KafkaProducer>,
) -> Result<Server, anyhow::Error> {
    let db_pool = web::Data::new(db_pool.clone());
    let wallet_repo = web::Data::new(WalletRepository::new(db_pool.get_ref().clone()));
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let kafka = web::Data::new(kafka_producer);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::JsonConfig::default().limit(262_144))
            .app_data(web::PayloadConfig::default().limit(10_485_760))
            // Core
            .route("/", web::get().to(home))
            .route("/health", web::get().to(health_check))
            // UI
            .route("/ui", web::get().to(dashboard))
            // Wallet API
            .route("/wallets", web::post().to(create_wallet))
            .route("/wallets/{id}", web::get().to(get_wallet))
            .route("/wallets/{id}/fund", web::post().to(fund_wallet))
            .route("/wallets/{id}/transfer", web::post().to(transfer))
            .route("/users/{user_id}/wallets", web::get().to(list_user_wallets))
            // App data
            .app_data(db_pool.clone())
            .app_data(wallet_repo.clone())
            .app_data(base_url.clone())
            .app_data(kafka.clone())
            .app_data(web::Data::new(HmacSecret(hmac_secret.clone())))
    })
    .listen(listener)?
    .run();

    Ok(server)
}

async fn dashboard() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("ui/dashboard.html"))
}

#[derive(Clone, Deserialize)]
pub struct HmacSecret(pub SecretString);
