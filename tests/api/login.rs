use crate::helpers::spawn_app;
use std::collections::HashSet;
use reqwest::header::HeaderValue;

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "username",
        "password": "username",
    });
    let response = app.post_login(&login_body).await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");

    // Assert that error message is shown when redirected to login page
    let response = app.get_login().await;
    let html_page = response.text().await.unwrap();
    assert!(html_page.contains("<p><i>Authentication failed</i></p>"));

    // Assert that error message is no longer shown after reloading
    let response = app.get_login().await;
    let html_page = response.text().await.unwrap();
    assert!(!html_page.contains("<p><i>Authentication failed</i></p>"));
}
