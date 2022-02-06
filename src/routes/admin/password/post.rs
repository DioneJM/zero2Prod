use actix_web::{HttpResponse, web};
use secrecy::{Secret, ExposeSecret};
use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};
use actix_web_flash_messages::FlashMessage;
use crate::routes::admin::dashboard::get_username;
use crate::startup::DbConnectionKind;
use crate::authentication::{Credentials, validate_credentials, AuthError};

static MINIMUM_PASSWORD_LENGTH: u8 = 8;
static MAXIMUM_PASSWORD_LENGTH: u8 = 128;


#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>
}

pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    database: web::Data<DbConnectionKind>
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = session.get_user_id().map_err(e500)?;

    if user_id.is_none() {
        return Ok(see_other("/login"));
    }

    let user_id = user_id.unwrap();

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error("You entered two different new password - the field values must match")
            .send();
        return Ok(see_other("/admin/password"));
    }

    if !(MINIMUM_PASSWORD_LENGTH..MAXIMUM_PASSWORD_LENGTH).contains(&(form.new_password.expose_secret().chars().count() as u8)) {
        FlashMessage::error("The new password must be between 12 and 128 characters long")
            .send();
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(user_id, &database).await.map_err(e500)?;
    let credentials = Credentials {
        username,
        password: form.0.current_password
    };
    if let Err(validation_error) = validate_credentials(credentials, &database).await {
        return match validation_error {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect")
                    .send();
               Ok(see_other("/admin/password"))
            },
            AuthError::UnexpectedError(_) => Err(e500(validation_error).into())
        }
    }

    Ok(HttpResponse::Ok().finish())
}