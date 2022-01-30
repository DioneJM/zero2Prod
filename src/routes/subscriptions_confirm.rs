use actix_web::{HttpResponse, web, ResponseError};
use crate::startup::DbConnectionKind;
use crate::domain::NewSubscriber;
use uuid::Uuid;
use sqlx::{Transaction, Postgres};
use std::fmt::Formatter;
use std::error::Error;
use std::any::TypeId;
use crate::routes::subscriptions::SubscribeError;
use actix_web::http::StatusCode;
use actix_web::body::BoxBody;
use sqlx::error::Error::Configuration;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(Debug)]
pub enum ConfirmError {
    PoolError(sqlx::Error),
    ResourceNotFound(sqlx::Error),
    TransactionCommitError(sqlx::Error),
    StatusUpdateError(sqlx::Error),
}

impl std::fmt::Display for ConfirmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfirmError::PoolError(_) => write!(f, "Failed to acquire a postgress connection"),
            ConfirmError::ResourceNotFound(_) => write!(f, "Resource does not exist"),
            ConfirmError::TransactionCommitError(_) => write!(f, "Failed to commit SQL transaction to store a new subscriber"),
            ConfirmError::StatusUpdateError(_) => write!(f, "Failed to set status of subscriber")
        }
    }
}

impl std::error::Error for ConfirmError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfirmError::PoolError(e) => Some(e),
            ConfirmError::ResourceNotFound(e) => Some(e),
            ConfirmError::TransactionCommitError(e) => Some(e),
            ConfirmError::StatusUpdateError(e) => Some(e),
        }
    }
}

impl ResponseError for ConfirmError {
    fn status_code(&self) -> StatusCode {
        match self {
            ConfirmError::ResourceNotFound(_) => StatusCode::BAD_REQUEST,
            ConfirmError::PoolError(_) |
            ConfirmError::TransactionCommitError(_) |
            ConfirmError::StatusUpdateError(_)=> StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(
name = "Confirm a pending subscriber",
skip(connection, params)
)]
pub async fn confirm(
    connection: web::Data<DbConnectionKind>,
    params: web::Query<Parameters>,
) -> Result<HttpResponse, ConfirmError> {
    let subscription_token = params.0.subscription_token;

    let mut transaction = connection
        .begin()
        .await
        .map_err(ConfirmError::PoolError)?;

    let subscriber_id = get_subscriber_id_from_token(
        &mut transaction,
        subscription_token,
    ).await
        .map_err(ConfirmError::ResourceNotFound)?;

    match subscriber_id {
        None => Ok(HttpResponse::Unauthorized().finish()),
        Some(id) => {
            confirm_subscriber(&mut transaction, id)
                .await
                .map_err(ConfirmError::StatusUpdateError)?;
            transaction.commit().await.map_err(ConfirmError::TransactionCommitError)?;
            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[tracing::instrument(
name = "Set subscriber status to confirm",
skip(transaction, subscriber_id)
)]
pub async fn confirm_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = 'confirmed'
        WHERE id = $1
        "#,
        subscriber_id
    )
        .execute(transaction)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(())
}

#[tracing::instrument(
name = "Retrieving subscriber id from token",
skip(transaction, subscription_token)
)]
pub async fn get_subscriber_id_from_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscription_token: String,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token
    )
        .fetch_optional(transaction)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(result.map(|res| res.subscriber_id))
}
