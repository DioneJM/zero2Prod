use actix_web::{HttpResponse, web, ResponseError};
use chrono::Utc;
use uuid::Uuid;

use crate::FormData;
use crate::startup::{DbConnectionKind, ApplicationBaseUrl};
use crate::domain::{NewSubscriber};
use std::convert::{TryFrom, TryInto};
use crate::email_client::EmailClient;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use sqlx::{Transaction, Postgres};
use std::fmt::{Display, Formatter};
use std::error::Error;
use std::any::TypeId;
use actix_web::http::StatusCode;
use actix_web::body::BoxBody;

pub struct StoreTokenError(sqlx::Error);

impl Display for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while trying to store a subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

// Error that can occur when creating a new subscriber
#[derive(Debug)]
pub enum SubscribeError {
    ValidationError(String),
    PoolError(sqlx::Error),
    InsertSubscriberError(sqlx::Error),
    TransactionCommitError(sqlx::Error),
    StoreTokenError(StoreTokenError),
    SendEmailError(reqwest::Error)
}

impl From<String> for SubscribeError {
    fn from(e: String) -> Self {
        Self::ValidationError(e)
    }
}

impl From<StoreTokenError> for SubscribeError {
    fn from(e: StoreTokenError) -> Self {
        Self::StoreTokenError(e)
    }
}

impl From<reqwest::Error> for SubscribeError {
    fn from(e: reqwest::Error) -> Self {
        Self::SendEmailError(e)
    }
}

impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscribeError::ValidationError(e) => write!(f, "{}", e),
            SubscribeError::StoreTokenError(_) => write!(f, "Failed to store confirmation token for new subscriber"),
            SubscribeError::SendEmailError(_) => write!(f, "Failed to send confirmation email"),
            SubscribeError::PoolError(_) => write!(f, "Failed to acquire a postgress connection"),
            SubscribeError::InsertSubscriberError(_) => write!(f, "Failed to insert new subscriber in the database"),
            SubscribeError::TransactionCommitError(_) => write!(f, "Failed to commit SQL transaction to store a new subscriber")
        }
    }
}

impl std::error::Error for SubscribeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SubscribeError::ValidationError(_) => None,
            SubscribeError::StoreTokenError(e) =>Some(e),
            SubscribeError::SendEmailError(e) =>Some(e),
            SubscribeError::PoolError(e) => Some(e),
            SubscribeError::InsertSubscriberError(e) => Some(e),
            SubscribeError::TransactionCommitError(e) => Some(e),
        }
    }
}
impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::PoolError(_) |
            SubscribeError::InsertSubscriberError(_) |
            SubscribeError::TransactionCommitError(_) |
            SubscribeError::StoreTokenError(_) |
            SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[tracing::instrument(
name = "Adding a new subscriber",
skip(form, connection, email_client, base_url),
fields(
subscriber_email = % form.email,
subscriber_name = % form.name
)
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    connection: web::Data<DbConnectionKind>, // connection is passed from application state
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber: NewSubscriber = form.0.try_into()?;

    // We create a transaction at the endpoint level so that all of the DB updates
    // that is made below will all be committed/rollbacked together (handled atomically)
    let mut transaction = connection
        .begin()
        .await
        .map_err(SubscribeError::PoolError)?;
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email = %new_subscriber.email.as_ref(),
        subscriber_name = %new_subscriber.name.as_ref()
    );

    let _request_span_guard = request_span.enter();

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .map_err(SubscribeError::InsertSubscriberError)?;

    let subscription_token = generate_subscription_token();

    store_token(&mut transaction, &subscriber_id, &subscription_token).await?;
    transaction.commit().await.map_err(SubscribeError::TransactionCommitError)?;

   send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    ).await?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
name = "Send a confirmation email to a new subscriber",
skip(email_client, new_subscriber, base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{base_url}/subscriptions/confirm?subscription_token={token}",
        base_url = base_url,
        token = subscription_token
    );
    let html_body = &format!(
        "<h1>Welcome</h1><br/>Welcome to our newsletter! Click <a href=\"{url}\">here</a> to confirm your subscription.",
        url = confirmation_link
    );
    let text_body = &format!("Welcome to our newsletter!\nVisit {url} to confirm your subscription", url = confirmation_link);
    email_client.send_email(
        &new_subscriber.email,
        "Welcome!",
        html_body,
        text_body,
    )
        .await
}

#[tracing::instrument(
name = "Saving new subscriber in DB",
skip(new_subscriber, transaction),
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
        .execute(transaction)
        .await
        .map_err(|e| {
            e
        })?;
    Ok(subscriber_id)
}

#[tracing::instrument(
name = "Saving subscription_token of new subscriber",
    skip(transaction, subscriber_id, subscription_token),
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: &Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id,
    )
        .execute(transaction)
        .await
        .map_err(|e| {
            StoreTokenError(e)
        })?;

    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}