use crate::helpers::spawn_app;
use wiremock::{Mock, ResponseTemplate, MockServer};
use wiremock::matchers::{any, path, method};
use reqwest::Url;


#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    let body = "name=Dione&email=dione%40email.com";

    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token")
        .execute(&app.connection)
        .await
        .unwrap();

    let client = reqwest::Client::new();

    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to submit subscription information");

    assert_eq!(response.status().as_u16(), 500);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;
    let body = "name=Dione&email=dione%40email.com";
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let client = reqwest::Client::new();
    client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to submit subscription information");

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_link = app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_link.html)
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = spawn_app().await;

    let client = reqwest::Client::new();
    let name = "Dione";
    let email = "dionemorales@outlook.com";
    let body = format!("name={name}&email={email}", name = name, email = email);

    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to submit subscription information");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.connection)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.name, name);
    assert_eq!(saved.email, email);
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_for_valid_form_data() {
    let app = spawn_app().await;
    let body = "name=Dione&email=dione%40email.com";

    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let client = reqwest::Client::new();

    let _ = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to submit subscription information");
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_with_link() {
    let app = spawn_app().await;
    let body = "name=Dione&email=dione%40email.com";

    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let client = reqwest::Client::new();

    let _ = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to submit subscription information");

    // Get the first intercepted request
    let intercepted_requests = &app.email_server.received_requests().await.unwrap();
    let email_request = &intercepted_requests[0];
    let confirmation_links = app.get_confirmation_links(email_request);
    let html_link = confirmation_links.html;
    let text_link = confirmation_links.plain_text;
    assert_eq!(html_link, text_link)
}

#[tokio::test]
async fn subscribe_returns_400_when_missing_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=Dione", "missing the email"),
        ("email=dionemorales@outlook.com", "missing name"),
        ("", "missing name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "applications/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        )
    }
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;
    let body = "name=Dione&email=dione%40email.com";

    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let client = reqwest::Client::new();

    client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to submit subscription information");

    let saved_subscriber = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.connection)
        .await
        .expect("Failed to get saved subscription");

    assert_eq!(saved_subscriber.email, "dione@email.com");
    assert_eq!(saved_subscriber.name, "Dione");
    assert_eq!(saved_subscriber.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
}