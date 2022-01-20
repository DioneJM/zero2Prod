use once_cell::sync::Lazy;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use uuid::Uuid;
use zero2prod::startup::{DbConnectionKind, Application, get_database_connection};
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

    let configuration = {
        let mut config = get_configuration()
            .expect("Failed to read config file");

        config.database.database_name = Uuid::new_v4().to_string();
        config.application.port = 0;
        config
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");
    let address = format!("http:127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());
    // We return the application address to the caller!
    TestApp {
        address,
        connection: get_database_connection(&configuration.database)
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
