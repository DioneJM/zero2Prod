use std::net::TcpListener;
use zero2prod::startup::{run};
use zero2prod::configuration::get_configuration;
use sqlx::{PgPool};
use zero2prod::startup::DbConnectionKind;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout
    );
    init_subscriber(subscriber);

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
