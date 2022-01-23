use actix_web::{HttpResponse, Responder, web};
use chrono::Utc;
use uuid::Uuid;

use crate::FormData;
use crate::startup::{DbConnectionKind, ApplicationBaseUrl};
use crate::domain::{NewSubscriber};
use std::convert::{TryFrom, TryInto};
use crate::email_client::EmailClient;

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
    base_url: web::Data<ApplicationBaseUrl>
) -> impl Responder {
    let new_subscriber: NewSubscriber = match form.0.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish()
    };
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email = %new_subscriber.email.as_ref(),
        subscriber_name = %new_subscriber.name.as_ref()
    );

    let _request_span_guard = request_span.enter();

    if insert_subscriber(&connection, &new_subscriber).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0
    ).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str
) -> Result<(), reqwest::Error> {
    let subscription_token = "mytoken";
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
        new_subscriber.email,
        "Welcome!",
        html_body,
        text_body,
    )
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber in DB",
    skip(new_subscriber, connection),
)]
pub async fn insert_subscriber(
    connection: &DbConnectionKind,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
        .execute(connection)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(())
}