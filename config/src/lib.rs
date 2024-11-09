use embedded_svc::wifi::AuthMethod;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub networks: Vec<WifiNetwork>,
    pub home_assistant_config: HomeAssistantConfig,
}
impl Config {
    pub fn load_or_panic() -> Config {
        let config_text = include_str!("../../deployment_config.toml");
        toml::from_str(config_text).expect("Failed to parse deployment config")
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub auth_method: AuthMethod,
    #[serde(default)]
    pub password: String,
}
#[derive(Deserialize, Debug, Clone)]
pub struct HomeAssistantConfig {
    pub base_url: String,
    pub access_token: String,
    pub person_entity: String,
    pub office_zone: String,
}
