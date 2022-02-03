use actix_web::{HttpResponse, web};
use actix_web::http::header::ContentType;
use crate::startup::HmacSecret;
use sha2::Sha256;
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;

pub type HmacSha256 = Hmac<Sha256>;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String
}

impl QueryParams {
    fn  verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag  = hex::decode(self.tag)?;
        let tag = tag.as_slice();
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
            secret.0.expose_secret().as_bytes()
        ).unwrap();

        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

pub async fn login_form(
    params: Option<web::Query<QueryParams>>,
    secret: web::Data<HmacSecret>
) -> HttpResponse {
    let error_message = match params {
        Some(params) => match params.0.verify(&secret) {
            Ok(error) => {
                format!("<p><i>{error}</i></p>", error = htmlescape::encode_minimal(&error))
            }
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to verify query parameters using the HMAC tag"
                );
                "".into()
            }
        },
        None => "".to_string()
    };
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>
<body>
    {error}
    <form action="/login" method="post">
        <label>
            Username
            <input type="text"
                   placeholder="Enter Username"
                   name="username"
            >
        </label>
        <label>
            Password
            <input type="password"
                   placeholder="Enter Password"
                   name="password"
            >
        </label>
        <button type="submit">Login</button>
    </form>
</body>
</html>
            "#,
            error = error_message
        ))
}
