use actix_web::{HttpResponse, web};
use crate::startup::DbConnectionKind;
use crate::domain::NewSubscriber;
use uuid::Uuid;
use sqlx::{Transaction, Postgres};

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String
}

#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(connection, params)
)]
pub async fn confirm(
    connection: web::Data<DbConnectionKind>,
    params: web::Query<Parameters>
) -> HttpResponse {
    let subscription_token = params.0.subscription_token;

    let mut transaction = match connection.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };

    let subscriber_id = match get_subscriber_id_from_token(
        &mut transaction,
        subscription_token
    ).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };

    match subscriber_id {
        None => HttpResponse::Unauthorized().finish(),
        Some(id) => {
            if confirm_subscriber(&mut transaction, id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            if transaction.commit().await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
    }
}

#[tracing::instrument(
    name = "Set subscriber status to confirm",
    skip(transaction, subscriber_id)
)]

pub async fn confirm_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid
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
    subscription_token: String
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
