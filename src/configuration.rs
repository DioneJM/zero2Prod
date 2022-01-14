use secrecy::{Secret, ExposeSecret};
use std::convert::{TryInto, TryFrom};

pub enum Environment {
    Local,
    Production
}
impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!("{} is not a supported environment. Use 'local' or 'production'", other))
        }
    }
}


#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{username}:{password}@{host}:{port}/{database_name}",
            username = self.username,
            password = self.password.expose_secret(),
            host = self.host,
            port = self.port,
            database_name = self.database_name
        ))
    }
    pub fn connection_string_without_db_name(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{username}:{password}@{host}:{port}",
            username = self.username,
            password = self.password.expose_secret(),
            host = self.host,
            port = self.port
        ))
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let mut settings = config::Config::default();
    let base_path = std::env::current_dir().expect("Failed to determine current directory");
    let config_directory = base_path.join("configuration");

    // Read config file
    settings.merge(config::File::from(config_directory.join("base")).required(true))?;

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");

    settings.merge(
        config::File::from(config_directory.join(environment.as_str())).required(true)
    ).expect("Failed to read environment file");

    // Parse config file into Setting struct

    settings.try_into()
}