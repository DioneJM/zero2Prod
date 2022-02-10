use actix_web::{HttpResponse, web};
use crate::startup::DbConnectionKind;
use std::fmt::Formatter;
use crate::routes::error_chain_fmt;
use crate::email_client::EmailClient;
use crate::domain::subscriber_email::SubscriberEmail;
use anyhow::Context;
use crate::session_state::TypedSession;
use crate::utils::{see_other, e500};
use crate::routes::admin::dashboard::get_username;
use validator::HasLen;
use actix_web_flash_messages::FlashMessage;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    html_content: String,
    text_content: String,
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

#[tracing::instrument(
name = "Publish a newsletter issue",
skip(form, database, email_client, session)
fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub async fn publish_newsletter(
    form: web::Form<BodyData>,
    database: web::Data<DbConnectionKind>,
    email_client: web::Data<EmailClient>,
    session: TypedSession
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = session.get_user_id().map_err(e500)?;
    if user_id.is_none() {
        return Ok(see_other("/login"))
    }
    let user_id = user_id.unwrap();
    let username = get_username(user_id, &database).await.map_err(e500)?;
    tracing::Span::current().record(
        "username",
        &tracing::field::display(&username),
    );
    tracing::Span::current().record(
        "user_id",
        &tracing::field::display(&user_id),
    );
    let confirmed_subscribers = get_confirmed_subscribers(&database).await.map_err(e500)?;
    tracing::info!("{}", &format!("# confirmed subscribers: {}", confirmed_subscribers.length()));
    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client.send_email(
                    &subscriber.email,
                    &form.0.title,
                    &form.0.html_content,
                    &form.0.text_content,
                ).await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    }).map_err(e500)?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber as stored details are invalid"
                )
            }
        }
    }
    FlashMessage::info("Emails have been sent!").send();
    Ok(see_other("/admin/newsletter"))
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