use actix_web::{HttpResponse, web};
use actix_web::http::header::LOCATION;
use secrecy::{Secret, ExposeSecret};
use crate::authentication::{Credentials, validate_credentials, AuthError};
use crate::startup::{DbConnectionKind, HmacSecret};
use std::fmt::Formatter;
use crate::routes::error_chain_fmt;
use actix_web::error::InternalError;
use sha2::Sha256;
use hmac::{Hmac, Mac};

pub type HmacSha256 = Hmac<Sha256>;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(
skip(form, database, secret),
fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub async fn login(
    form: web::Form<FormData>,
    database: web::Data<DbConnectionKind>,
    secret: web::Data<HmacSecret>
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &database).await {
        Ok(user_id) => {
            tracing::Span::current()
                .record("user_id", &tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            let login_error = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into())
            };
            let encoded_error = urlencoding::Encoded::new(login_error.to_string());
            let query_string = format!("error={}", encoded_error);

            let hmac_tag = {
                let mut mac = HmacSha256::new_from_slice(
                    secret.0.expose_secret().as_bytes()
                ).unwrap();
                mac.update(query_string.as_bytes());
                mac.finalize().into_bytes()
            };
            let response = HttpResponse::SeeOther()
                .insert_header((
                    LOCATION,
                    format!("/login?{}&tag={:x}",
                            query_string,
                            hmac_tag)
                ))
                .finish();
            Err(InternalError::from_response(login_error, response))
        }
    }
}