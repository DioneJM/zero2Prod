pub mod configuration;
pub mod routes;
pub mod startup;
pub mod telemetry;
pub mod domain;
pub mod email_client;
pub mod authentication;
pub mod session_state;
pub mod utils;
pub mod idempotency;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}