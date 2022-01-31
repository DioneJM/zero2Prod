use once_cell::sync::Lazy;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use uuid::Uuid;
use zero2prod::startup::{DbConnectionKind, Application, get_database_connection};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use zero2prod::telemetry::{init_subscriber, get_subscriber};
use wiremock::MockServer;

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

pub struct ConfirmationLinks {
    pub html: reqwest:: Url,
    pub plain_text: reqwest::Url
}

pub struct TestApp {
    pub address: String,
    pub connection: DbConnectionKind,
    pub email_server: MockServer,
    pub port: u16
}

impl TestApp {
    pub fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request
    ) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|link| { *link.kind() == linkify::LinkKind::Url })
                .collect();
            assert_eq!(links.len(), 1);

            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port));
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks {
            html,
            plain_text
        }
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/newsletters", &self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to POST newsletters endpoint")
    }
}


pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;
    let configuration = {
        let mut config = get_configuration()
            .expect("Failed to read config file");

        config.database.database_name = Uuid::new_v4().to_string();
        config.application.port = 0;
        config.email_client.base_url = email_server.uri();
        config
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");
    let application_port = application.port();
    let address = format!("http:127.0.0.1:{}", application_port);
    let _ = tokio::spawn(application.run_until_stopped());
    // We return the application address to the caller!
    TestApp {
        address,
        port: application_port,
        connection: get_database_connection(&configuration.database),
        email_server
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
