use std::net::TcpListener;

use actix_web::{App, HttpServer, web};
use actix_web::dev::Server;
use sqlx::{PgPool};

use crate::routes;
use tracing_actix_web::TracingLogger;
use crate::email_client::EmailClient;
use actix_web::web::Data;
use crate::configuration::{Settings, DatabaseSettings};
use sqlx::postgres::PgPoolOptions;
use crate::domain::subscriber_email::SubscriberEmail;
use secrecy::Secret;

pub type DbConnectionKind = PgPool;

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);

pub struct Application {
    port: u16,
    server: Server
}

impl Application {
    pub async fn build(config: Settings) -> Result<Self, std::io::Error> {
        let db_connection_pool: DbConnectionKind = get_database_connection(&config.database);

        let sender_email: SubscriberEmail = config.email_client.sender()
            .expect("Invalid email found in config");

        let timeout = std::time::Duration::from_millis(config.email_client.timeout_milliseconds);
        let email_client = EmailClient::new(
            config.email_client.base_url,
            sender_email,
            config.email_client.authorization_token,
            timeout
        );

        let address = format!(
            "{address}:{port}",
            address = config.application.host,
            port = config.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            db_connection_pool,
            email_client,
            config.application.base_url,
            config.application.hmac_secret
        )?;

        Ok( Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub struct ApplicationBaseUrl(pub String);

pub fn get_database_connection(config: &DatabaseSettings) -> DbConnectionKind {
    PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.with_db())
}

pub fn run(
    listener: TcpListener,
    connection: DbConnectionKind,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>
) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(connection);
    let email_client = Data::new(email_client);
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health", web::get().to(routes::health_check::health_check))
            .route("/", web::get().to(routes::home::home))
            .route("/subscriptions", web::post().to(routes::subscriptions::subscribe))
            .route("/subscriptions/confirm", web::get().to(routes::subscriptions_confirm::confirm))
            .route("/newsletters", web::post().to(routes::newsletters::publish_newsletter))
            .route("/login", web::get().to(routes::login::get::login_form))
            .route("/login", web::post().to(routes::login::post::login))
            .app_data(connection.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(Data::new(HmacSecret(hmac_secret.clone())))
    })
        .listen(listener)?
        .run();
    Ok(server)
}
