use actix_web::{HttpResponse, ResponseError, web};
use crate::startup::DbConnectionKind;
use std::fmt::Formatter;
use crate::routes::error_chain_fmt;
use actix_web::http::StatusCode;
use actix_web::body::BoxBody;
use std::any::TypeId;
use crate::email_client::EmailClient;
use std::convert::TryInto;
use crate::domain::subscriber_email::SubscriberEmail;
use anyhow::Context;

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
    UnexpectedError(#[from] anyhow::Error)
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    database: web::Data<DbConnectionKind>,
    email_client: web::Data<EmailClient>
) -> Result<HttpResponse, PublishError> {
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
        email: String
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