use anyhow::{bail, Result};
use embedded_svc::wifi::AuthMethod;
use nonempty::NonEmpty;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub networks: NonEmpty<WifiNetwork>,
    pub home_assistant_config: HomeAssistantConfig,
    pub flickering_gps_leds: Vec<FlickeringGpsLed>,
}
impl Config {
    pub fn load_or_panic() -> Self {
        let config_text = include_str!("../../deployment_config.toml");
        let cfg: Self = toml::from_str(config_text).expect("Failed to parse deployment config");
        cfg.validate().unwrap();
        return cfg;
    }
    fn validate(&self) -> Result<()> {
        let mut seen_gpio_pins = HashSet::<i32>::new();
        for cfg in &self.flickering_gps_leds {
            if !seen_gpio_pins.insert(cfg.gpio_pin) {
                bail!("Pin '{}' configured multiple times.", cfg.gpio_pin);
            }
        }
        Ok(())
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
}
#[derive(Deserialize, Debug, Clone)]
pub struct FlickeringGpsLed {
    pub entity: String,
    pub gps_zone: String,
    pub gpio_pin: i32,
    pub min_brightness: f32,
    pub max_brightness: f32,
}
