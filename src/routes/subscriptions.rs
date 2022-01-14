use actix_web::{HttpResponse, Responder, web};
use chrono::Utc;
use uuid::Uuid;

use crate::FormData;
use crate::startup::DbConnectionKind;

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
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "Adding a new subscriber",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
    );

    let _request_span_guard = request_span.enter();
    tracing::info!("Saving new subscriber details - name: {} email: {}", form.name, form.email);

    match insert_subscriber(&connection, &form).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_e) => HttpResponse::InternalServerError().finish()
    }
}

#[tracing::instrument(
    name = "Saving new subscriber in DB",
    skip(form, connection),
)]
pub async fn insert_subscriber (
    connection: &DbConnectionKind,
    form: &FormData
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
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