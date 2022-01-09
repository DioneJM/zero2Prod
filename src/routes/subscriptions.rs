use actix_web::{web, Responder, HttpResponse};
use crate::FormData;

pub async fn subscribe(form: web::Form<FormData>) -> impl Responder {
    let hello = format!("Hello {name}", name = form.name);
    HttpResponse::Ok().body(hello)
}