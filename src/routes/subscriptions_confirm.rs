use actix_web::{HttpResponse, web};
use crate::startup::DbConnectionKind;
use crate::domain::NewSubscriber;
use uuid::Uuid;

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
    let subscriber_id = match get_subscriber_id_from_token(
        &connection,
        subscription_token
    ).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };

    match subscriber_id {
        None => HttpResponse::Unauthorized().finish(),
        Some(id) => {
            if confirm_subscriber(&connection, id).await.is_err() {
                return HttpResponse::InternalServerError().finish()
            }
            HttpResponse::Ok().finish()
        }
    }
}

pub async fn confirm_subscriber(
    connection: &DbConnectionKind,
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
        .execute(connection)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(())
}

#[tracing::instrument(
    name = "Retrieving subscriber id from token",
    skip(connection, subscription_token)
)]
pub async fn get_subscriber_id_from_token(
    connection: &DbConnectionKind,
    subscription_token: String
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT subscriber_id FROM subscription_tokens
        WHERE subscription_token = $1
        "#,
        subscription_token
    )
        .fetch_optional(connection)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(result.map(|res| res.subscriber_id))
}
