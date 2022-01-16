pub mod configuration;
pub mod routes;
pub mod startup;
pub mod telemetry;
pub mod domain;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}