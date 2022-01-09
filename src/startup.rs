use std::net::TcpListener;
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use crate::routes;

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(routes::health_check::health_check))
            .route("/subscriptions", web::post().to(routes::subscriptions::subscribe))
    })
        .listen(listener)?
        .run();
    Ok(server)
}
