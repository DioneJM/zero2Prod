use actix_web::{HttpResponse, web};
use actix_web::http::header::ContentType;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: Option<String>
}

pub async fn login_form(params: web::Query<QueryParams>) -> HttpResponse {
    let error_message = match params.0.error {
        Some(error) => String::from(error),
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
