pub mod configuration;
pub mod routes;
pub mod startup;
pub mod telemetry;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}