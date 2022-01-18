use once_cell::sync::Lazy;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use uuid::Uuid;
use std::net::TcpListener;
use zero2prod::startup::{DbConnectionKind, run};
use zero2prod::domain::subscriber_email::SubscriberEmail;
use zero2prod::email_client::EmailClient;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use zero2prod::telemetry::{init_subscriber, get_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {

    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::stdout
        );
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::sink
        );
        init_subscriber(subscriber);
    }

});

pub struct TestApp {
    pub address: String,
    pub connection: DbConnectionKind
}


pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let mut config = get_configuration()
        .expect("Failed to read config file");

    config.database.database_name = Uuid::new_v4().to_string();
    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = listener.local_addr()
        .unwrap()
        .port();
    let db_connection_pool: DbConnectionKind = configure_database(&config.database).await;

    let sender_email: SubscriberEmail = config.email_client.sender()
        .expect("Invalid email found in config");

    let timeout = config.email_client.timeout();
    let email_client = EmailClient::new(
        config.email_client.base_url,
        sender_email,
        config.email_client.authorization_token,
        timeout
    );


    let server = run(
        listener,
        db_connection_pool.clone(),
        email_client
    )
        .expect("Failed to bind address");
    let _ = tokio::spawn(server);
    // We return the application address to the caller!
    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        connection: db_connection_pool
    }
}

async fn configure_database(config: &DatabaseSettings) -> DbConnectionKind {
    let _connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to DB")
        .execute(format!(
            r#"
            CREATE DATABASE "{db_name}";
            "#,
            db_name = config.database_name
        ).as_str())
        .await
        .expect("Failed to create DB");

    // migrate
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to DB when creating connection pool");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate DB");

    connection_pool
}
