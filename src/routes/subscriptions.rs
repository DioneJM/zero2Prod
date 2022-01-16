use actix_web::{HttpResponse, Responder, web};
use chrono::Utc;
use uuid::Uuid;

use crate::FormData;
use crate::startup::DbConnectionKind;
use crate::domain::{SubscriberName, NewSubscriber};

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, connection),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    connection: web::Data<DbConnectionKind>, // connection is passed from application state
) -> impl Responder {
    let subscriber_name = SubscriberName::parse(form.name.clone());
    let new_subscriber: NewSubscriber = NewSubscriber {
        email: form.email.clone(),
        name: subscriber_name
    };
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email = %new_subscriber.email,
        subscriber_name = %new_subscriber.name.as_ref()
    );

    let _request_span_guard = request_span.enter();
    tracing::info!("Saving new subscriber details - name: {} email: {}", form.name, form.email);

    match insert_subscriber(&connection, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_e) => HttpResponse::InternalServerError().finish()
    }
}

#[tracing::instrument(
    name = "Saving new subscriber in DB",
    skip(new_subscriber, connection),
)]
pub async fn insert_subscriber (
    connection: &DbConnectionKind,
    new_subscriber: &NewSubscriber
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email,
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