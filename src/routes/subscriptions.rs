use actix_web::{HttpResponse, Responder, web};
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

    // We create a transaction at the endpoint level so that all of the DB updates
    // that is made below will all be committed/rollbacked together (handled atomically)
    let mut transaction = match connection.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email = %new_subscriber.email.as_ref(),
        subscriber_name = %new_subscriber.name.as_ref()
    );

    let _request_span_guard = request_span.enter();

    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let subscription_token = generate_subscription_token();

    if store_token(&mut transaction, &subscriber_id, &subscription_token).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token
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
    base_url: &str,
    subscription_token: &str
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
        new_subscriber.email,
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
            tracing::error!("Failed to execute query: {:?}", e);
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
) -> Result<(), sqlx::Error> {
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
            tracing::error!("Failed to execute query: {:?}", e);
            e
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