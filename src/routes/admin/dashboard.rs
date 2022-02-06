use actix_web::{HttpResponse, web};
use uuid::Uuid;
use crate::startup::DbConnectionKind;
use anyhow::Context;
use crate::session_state::TypedSession;
use reqwest::header::LOCATION;
use crate::utils::e500;

pub async fn admin_dashboard(
    session: TypedSession,
    database: web::Data<DbConnectionKind>
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session
        .get_user_id()
        .map_err(e500)?
    {
        get_username(user_id, &database).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish()
        )
    };
    Ok(HttpResponse::Ok()
        .body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Admin dashboard</title>
            </head>
            <body>
                <p>Welcome {}!</p>
            </body>
            </html>
            "#,
            username
        )))
}

#[tracing::instrument(name = "Get username", skip(database))]
async fn get_username(
    user_id: Uuid,
    database: &DbConnectionKind
) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id
    )
        .fetch_one(database)
        .await
        .context("Failed to get username")?;
    Ok(row.username)
}