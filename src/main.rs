use std::net::TcpListener;
use zero2prod::startup::{run};
use zero2prod::configuration::get_configuration;
use zero2prod::startup::DbConnectionKind;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use sqlx::postgres::PgPoolOptions;

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

    run(listener, db_connection_pool)?.await
}
