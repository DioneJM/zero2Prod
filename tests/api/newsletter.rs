use crate::helpers::{spawn_app, TestApp, ConfirmationLinks};
use wiremock::{Mock, ResponseTemplate};
use wiremock::matchers::{any, path, method};
use uuid::Uuid;

#[tokio::test]
async fn newsletters_are_not_delivered_to_non_confirmed_subscribers() {
    let app = spawn_app().await;
    app.login_with_test_user().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body in plain text",
        "html_content": "<p>Newsletter body in HTML</p>"
    });

    let response = app.post_newsletters(&newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 200)
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscriber() {
    let app = spawn_app().await;
    app.login_with_test_user().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body in plain text",
        "html_content": "<p>Newsletter body in HTML</p>"
    });

    let response = app.post_newsletters(&newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 200)
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
                    "text_content": "text",
                    "html_content": "<p>html</p>"
            }),
            "missing title"
        ),
        (
            serde_json::json!({
                "title": "title"
            }),
            "missing content"
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;
        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        )
    }
}

#[tokio::test]
async fn requests_without_authorization_are_rejected() {
    let app = spawn_app().await;

    // no login is performed

    let newsletter_body = serde_json::json!({
        "title": "title",
        "html_content": "<p>html</p>",
        "text_content": "text"
    });
    let response = app.post_newsletters(&newsletter_body)
        .await;

    assert_eq!(response.status().as_u16(), 303);
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=Dione&email=dione%40email.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    let client = reqwest::Client::new();
    client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;

    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}