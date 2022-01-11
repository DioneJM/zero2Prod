use std::net::TcpListener;

use actix_web::{App, HttpServer, web};
use actix_web::dev::Server;
use sqlx::{PgPool};

use crate::routes;
use actix_web::middleware::Logger;

pub type DbConnectionKind = PgPool;

pub fn run(
    listener: TcpListener,
    connection: DbConnectionKind,
) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(connection);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/health", web::get().to(routes::health_check::health_check))
            .route("/subscriptions", web::post().to(routes::subscriptions::subscribe))
            .app_data(connection.clone())
    })
        .listen(listener)?
        .run();
    Ok(server)
}
