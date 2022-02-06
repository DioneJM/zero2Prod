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

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let incorrect_password_check = Uuid::new_v4().to_string();

    assert_ne!(new_password, incorrect_password_check);

    app.login_with_test_user().await;

    let response = app.post_change_password(&serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": new_password,
        "new_password_check": incorrect_password_check
    })).await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/admin/password");

    // Follow redirect
    let response = app.get_change_password().await;
    let html_page = response.text().await.unwrap();

    assert!(html_page.contains(
        "<p><i>You entered two different new password - the field values must match</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let incorrect_current_password = Uuid::new_v4().to_string();

    app.login_with_test_user().await;

    let response = app.post_change_password(&serde_json::json!({
        "current_password": incorrect_current_password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    })).await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/admin/password");

    // Follow redirect
    let response = app.get_change_password().await;
    let html_page = response.text().await.unwrap();

    assert!(html_page.contains(
        "<p><i>The current password is incorrect</i></p>"
    ));

}

#[tokio::test]
async fn new_password_cannot_be_too_short() {
    let app = spawn_app().await;
    let new_password = String::from("short_p");

    app.login_with_test_user().await;

    let response = app.post_change_password(&serde_json::json!({
        "current_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    })).await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/admin/password");

    // Follow redirect
    let response = app.get_change_password().await;
    let html_page = response.text().await.unwrap();

    assert!(html_page.contains(
        "<p><i>The new password must be between 12 and 128 characters long</i></p>"
    ));

}



fn assert_login_redirect(response: &Response) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}