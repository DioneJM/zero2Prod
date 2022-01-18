use std::net::TcpListener;
use zero2prod::startup::{run};
use zero2prod::configuration::get_configuration;
use zero2prod::startup::DbConnectionKind;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use sqlx::postgres::PgPoolOptions;
use zero2prod::email_client::EmailClient;
use zero2prod::domain::subscriber_email::SubscriberEmail;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    let config = get_configuration()
        .expect("Failed to read config file");
    let address = format!(
        "{address}:{port}",
        address = config.application.host,
        port = config.application.port
    );
    let listener = TcpListener::bind(address)?;
    let db_connection_pool: DbConnectionKind = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.database.with_db());

    let sender_email: SubscriberEmail = config.email_client.sender()
        .expect("Invalid email found in config");

    let timeout = config.email_client.timeout();
    let email_client = EmailClient::new(
        config.email_client.base_url,
        sender_email,
        config.email_client.authorization_token,
        timeout
    );

    run(listener, db_connection_pool, email_client)?.await
}
