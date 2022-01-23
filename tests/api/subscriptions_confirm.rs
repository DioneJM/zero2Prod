use crate::helpers::spawn_app;
use wiremock::{Mock, ResponseTemplate};
use wiremock::matchers::{path, method};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_400() {
    let app = spawn_app().await;

    let response = reqwest::get(
        &format!("{}/subscriptions/confirm", app.address)
    ).await.unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn clicking_on_confirmation_link_confirms_subscriber() {
    let app = spawn_app().await;
    let name = "Dione";
    let email = "dione%40email.com";
    let body = format!(
        "name={name}&email={email}",
        name = name,
        email = email
    );

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

    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.connection)
        .await
        .expect("Failed to retrieve subscriber");

    assert_eq!(saved.email, "dione@email.com");
    assert_eq!(saved.name, name);
    assert_eq!(saved.status, "confirmed");
}