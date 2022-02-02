use actix_web::{HttpResponse, ResponseError, web, http};
use crate::startup::DbConnectionKind;
use std::fmt::Formatter;
use crate::routes::error_chain_fmt;
use actix_web::http::StatusCode;
use actix_web::body::BoxBody;
use crate::email_client::EmailClient;
use crate::domain::subscriber_email::SubscriberEmail;
use anyhow::Context;
use secrecy::Secret;
use uuid::Uuid;
use secrecy::ExposeSecret;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use crate::telemetry::spawn_blocking_with_tracinig;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PublishError::AuthError(_) => StatusCode::UNAUTHORIZED
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let authenticate_value = http::header::HeaderValue::from_str(r#"Basic realm="publish"#).unwrap();
                response.headers_mut().insert(http::header::WWW_AUTHENTICATE, authenticate_value);
                response
            }
        }
    }
}

#[tracing::instrument(
name = "Publish a newsletter issue",
skip(body, database, email_client, request)
fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    database: web::Data<DbConnectionKind>,
    email_client: web::Data<EmailClient>,
    request: web::HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers())
        .map_err(PublishError::AuthError)?;
    tracing::Span::current().record(
        "username",
        &tracing::field::display(&credentials.username),
    );
    let user_id = validate_credentials(credentials, &database).await?;
    tracing::Span::current().record(
        "user_id",
        &tracing::field::display(&user_id),
    );
    let confirmed_subscribers = get_confirmed_subscribers(&database).await?;
    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client.send_email(
                    &subscriber.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                ).await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber as stored details are invalid"
                )
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

fn basic_authentication(headers: &http::header::HeaderMap) -> Result<Credentials, anyhow::Error> {
    let authorization = headers
        .get(http::header::AUTHORIZATION)
        .context("Authorization header missing")?
        .to_str()
        .context("Authorization header value no valid UTF8 string")?;

    let encoded_credentials = authorization
        .strip_prefix("Basic ")
        .context("Authorization scheme was not basic")?;

    let decoded_credentials_bytes = base64::decode_config(encoded_credentials, base64::STANDARD)
        .context("Failed to base64-decode credentials")?;

    let decoded_credentials = String::from_utf8(decoded_credentials_bytes)
        .context("Decoded credential is not valid UTF8 string")?;

    let mut credentials = decoded_credentials.splitn(2, ':');

    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth"))?
        .to_string();

    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth"))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(
name = "Retrieving CONFIRMED subscribers"
skip(database)
)]
async fn get_confirmed_subscribers(
    database: &DbConnectionKind
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    struct Row {
        email: String,
    }
    let rows = sqlx::query_as!(
        Row,
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#
    )
        .fetch_all(database)
        .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|row| match SubscriberEmail::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error))
        })
        .collect();
    Ok(confirmed_subscribers)
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, database))]
async fn validate_credentials(
    credentials: Credentials,
    database: &DbConnectionKind,
) -> Result<Uuid, PublishError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );
    if let Some((stored_user_id, stored_password_hash)) = get_user_credentials(credentials.username.as_str(), database)
        .await
        .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracinig(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
        .await
        .context("Failed to spawn block password hashing")
        .map_err(PublishError::UnexpectedError)??;

    user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username")))
}

#[tracing::instrument(name = "Get stored credentials", skip(username, database))]
async fn get_user_credentials(
    username: &str,
    database: &DbConnectionKind,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let user = sqlx::query!(
       r#"
       SELECT user_id, password_hash
       FROM users
       WHERE username = $1
       "#,
       username,
   ).fetch_optional(database)
        .await
        .context("Could not find user with  provided credentials")?
        .map(|u| (u.user_id, Secret::new(u.password_hash)));
    Ok(user)
}

#[tracing::instrument(name = "Verify password hash", skip(expected_password_hash, password_candidate))]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password_candidate.expose_secret().as_bytes(),
                         &expected_password_hash)
        .context("Invalid password")
        .map_err(PublishError::AuthError)
}