use actix_web::HttpResponse;
use secrecy::Secret;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>
}

pub async fn change_password() -> HttpResponse {
    HttpResponse::Ok().finish()
}