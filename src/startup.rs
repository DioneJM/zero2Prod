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

pub type DbConnectionKind = PgPool;

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
        let server = run(listener, db_connection_pool, email_client)?;

        Ok( Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_database_connection(config: &DatabaseSettings) -> DbConnectionKind {
    PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.with_db())
}

pub fn run(
    listener: TcpListener,
    connection: DbConnectionKind,
    email_client: EmailClient
) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(connection);
    let email_client = Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health", web::get().to(routes::health_check::health_check))
            .route("/subscriptions", web::post().to(routes::subscriptions::subscribe))
            .route("/subscriptions/confirm", web::get().to(routes::subscriptions_confirm::confirm))
            .app_data(connection.clone())
            .app_data(email_client.clone())
    })
        .listen(listener)?
        .run();
    Ok(server)
}
