use std::net::TcpListener;
use zero2prod::startup::{run};
use zero2prod::configuration::get_configuration;
use sqlx::{PgPool};
use zero2prod::startup::DbConnectionKind;
use tracing_subscriber::{EnvFilter, Registry};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    LogTracer::init().expect("Failed to set logger");
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new(
        "zero2prod".into(),
        std::io::stdout
    );

    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Failed to set subscriber");

    let config = get_configuration()
        .expect("Failed to read config file");
    let address = format!("127.0.0.1:{port}", port = config.application_port);
    let listener = TcpListener::bind(address)?;
    let db_connection_pool: DbConnectionKind = PgPool::connect(
        &config.database.connection_string())
        .await
        .expect("Failed to connect to DB");

    run(listener, db_connection_pool)?.await
}
