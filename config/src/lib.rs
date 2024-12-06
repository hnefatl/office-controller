#![no_std]

extern crate alloc;

use alloc::{collections::BTreeSet, string::String, vec::Vec};
use anyhow::{bail, Result};
use serde::{Serialize, Deserialize};

mod secure_string;
use secure_string::SecureString;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub networks: Vec<WifiNetwork>,
    pub home_assistant_config: HomeAssistantConfig,
    #[serde(default)]
    pub flickering_gps_leds: Vec<FlickeringGpsLed>,
}
impl Config {
    pub fn parse_or_panic(config_bytes: &[u8]) -> Self {
        let cfg: Self =
            postcard::from_bytes(config_bytes).expect("Failed to parse deployment config");
        cfg.validate().unwrap();
        return cfg;
    }
    fn validate(&self) -> Result<()> {
        let mut seen_gpio_pins = BTreeSet::<i32>::new();
        for cfg in &self.flickering_gps_leds {
            if !seen_gpio_pins.insert(cfg.gpio_pin) {
                bail!("Pin '{}' configured multiple times.", cfg.gpio_pin);
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct WifiNetwork {
    pub ssid: String,
    #[serde(default)]
    pub password: SecureString,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct HomeAssistantConfig {
    pub base_url: String,
    pub access_token: SecureString,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FlickeringGpsLed {
    pub entity: String,
    pub gps_zone: String,
    pub gpio_pin: i32,
    pub min_brightness: f32,
    pub max_brightness: f32,
}
