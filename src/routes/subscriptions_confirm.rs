use actix_web::{HttpResponse, web};

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String
}

#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(params)
)]
pub async fn confirm(params: web::Query<Parameters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}