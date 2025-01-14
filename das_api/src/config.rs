use crate::error::DasApiError;
use {
    figment::{providers::Env, Figment},
    serde::Deserialize,
};

#[derive(Deserialize)]
pub struct Config {
    pub database_url: String,
    pub metrics_port: Option<u16>,
    pub metrics_host: Option<String>,
    pub server_port: u16,
    pub env: Option<String>,
    pub cdn_prefix: Option<String>,
    pub enable_grand_total_query: Option<bool>,
    pub enable_collection_metadata: Option<bool>,
}

pub fn load_config() -> Result<Config, DasApiError> {
    Figment::new()
        .join(Env::prefixed("APP_"))
        .extract()
        .map_err(|config_error| DasApiError::ConfigurationError(config_error.to_string()))
}
