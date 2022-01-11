use actix_web::{HttpResponse, Responder, web};
use chrono::Utc;
use uuid::Uuid;

use crate::FormData;
use crate::startup::DbConnectionKind;

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
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
        .execute(connection.as_ref())
        .await
    {
        Ok(_) => {
            tracing::info!("New subscriber successfully saved");
            HttpResponse::Ok().finish()
        },
        Err(e) => {
            tracing::error!("Failed to execute query {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}