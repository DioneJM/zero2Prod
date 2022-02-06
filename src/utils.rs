use actix_web::HttpResponse;
use actix_web::http::header::LOCATION;

pub fn e500<T>(e: T) -> actix_web::error::InternalError<T> {
    actix_web::error::InternalError::from_response(
        e,
        HttpResponse::InternalServerError().finish(),
    )
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}