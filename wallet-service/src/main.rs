use wallet_service::{configuration::get_configuration, startup::Application, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_subscriber(telemetry::get_subscriber(
        "wallet-service".into(),
        "info".into(),
        std::io::stdout,
    ));

    let configuration = get_configuration().expect("Failed to read configuration");

    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;

    Ok(())
}
