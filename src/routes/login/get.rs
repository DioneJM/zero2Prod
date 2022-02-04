use actix_web::{HttpResponse, web};
use actix_web::http::header::ContentType;

pub async fn login_form(
    request: web::HttpRequest
) -> HttpResponse {
    let error_html = match request.cookie("_flash") {
        None => "".into(),
        Some(cookie) => {
            format!("<p><i>{error}</i></p>", error = cookie.value())
        }
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
            error = error_html
        ))
}
