use std::net::TcpListener;
use zero2prod::startup::{run, DbConnectionKind};
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use sqlx::{PgPool, Executor, ConnectOptions};
use uuid::Uuid;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use zero2prod::domain::subscriber_email::SubscriberEmail;
use zero2prod::email_client::EmailClient;

pub struct TestApp {
    pub address: String,
    pub connection: DbConnectionKind
}

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

async fn spawn_app() -> TestApp {
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

pub async fn configure_database(config: &DatabaseSettings) -> DbConnectionKind {
    let _connection = &config.without_db().connect()
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

//`tokio::test` is the testing equivalent of `tokio::main`.
// It also spares you from having to specify the `#[test]` attribute.
// To see how it works, run `cargo expand --test health_check`
#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        // Use the returned application address
        .get(&format!("{}/health", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some("A - OK".len() as u64), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();
    let name = "Dione";
    let email = "dionemorales@outlook.com";
    let body = format!("name={name}&email={email}", name = name, email = email);

    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to submit subscription information");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.connection)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.name, name);
    assert_eq!(saved.email, email);
}

#[tokio::test]
async fn subscribe_returns_400_when_missing_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=Dione", "missing the email"),
        ("email=dionemorales@outlook.com", "missing name"),
        ("", "missing name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "applications/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        )
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
}