use crate::helpers::spawn_app;
use uuid::Uuid;
use reqwest::Response;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let app = spawn_app().await;

    let response = app.get_change_password().await;

    assert_login_redirect(&response)
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_password() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();

    let response = app.post_change_password(&serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": &new_password,
        "new_password_check": &new_password
    })).await;

    assert_login_redirect(&response)
}


fn assert_login_redirect(response: &Response) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}