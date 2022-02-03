use secrecy::{Secret, ExposeSecret};
use crate::startup::DbConnectionKind;
use uuid::Uuid;
use crate::telemetry::spawn_blocking_with_tracinig;
use anyhow::Context;
use argon2::{PasswordHash, Argon2, PasswordVerifier};

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid Credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error)
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, database))]
pub async fn validate_credentials(
    credentials: Credentials,
    database: &DbConnectionKind,
) -> Result<Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );
    if let Some((stored_user_id, stored_password_hash)) = get_user_credentials(credentials.username.as_str(), database)
        .await
        .map_err(AuthError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracinig(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
        .await
        .context("Failed to spawn block password hashing")
        .map_err(AuthError::UnexpectedError)??;

    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username")))
}

#[tracing::instrument(name = "Get stored credentials", skip(username, database))]
async fn get_user_credentials(
    username: &str,
    database: &DbConnectionKind,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let user = sqlx::query!(
       r#"
       SELECT user_id, password_hash
       FROM users
       WHERE username = $1
       "#,
       username,
   ).fetch_optional(database)
        .await
        .context("Could not find user with  provided credentials")?
        .map(|u| (u.user_id, Secret::new(u.password_hash)));
    Ok(user)
}

#[tracing::instrument(name = "Verify password hash", skip(expected_password_hash, password_candidate))]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password_candidate.expose_secret().as_bytes(),
                         &expected_password_hash)
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}