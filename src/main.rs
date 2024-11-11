use config::FlickeringGpsLed;
use embassy_executor::{main, Spawner};
use embassy_time::Timer;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::{AnyOutputPin, PinDriver},
        prelude::*,
    },
    nvs::EspDefaultNvsPartition,
};
use log::info;
use std::sync::Arc;

mod homeassistant;
mod wifi;

// Disabled until I've got NVS encryption configured, don't want to leak WiFi keys via flash.
const USE_PERSISTENT_WIFI_STORAGE: bool = false;

#[main]
async fn main(spawner: Spawner) {
    let config = config::Config::load_or_panic();

    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take().unwrap();

    // Allow storing wifi tuning data and keys(?) in persistent storage, for better performance(?).
    // It at least stops an error on boot, although it's not runtime-critical.
    let wifi_nvs = USE_PERSISTENT_WIFI_STORAGE
        .then(EspDefaultNvsPartition::take)
        .transpose()
        .unwrap();

    let wifi_status = Arc::new(wifi::WifiStatus::new());
    spawner.must_spawn(wifi::repeatedly_connect_to_wifi(
        peripherals.modem,
        sysloop.clone(),
        wifi_nvs,
        wifi_status.clone(),
        config.networks,
    ));
    for cfg in config.flickering_gps_leds {
        spawner.must_spawn(flickering_gps_led_runner(
            wifi_status.clone(),
            config.home_assistant_config.clone(),
            cfg.clone(),
            // Config validation should mean we don't reuse GPIO pins.
            unsafe { AnyOutputPin::new(cfg.gpio_pin) },
        ));
    }
}

#[embassy_executor::task]
async fn flickering_gps_led_runner(
    wifi_status: Arc<wifi::WifiStatus>,
    ha_config: config::HomeAssistantConfig,
    led_config: FlickeringGpsLed,
    pin: AnyOutputPin,
) -> ! {
    let mut led = PinDriver::output(pin).unwrap();
    loop {
        // TODO: make a generic "loop callable while wifi connected" wrapper? will require 'static callables
        wifi_status.wait_until_connected().await;

        match homeassistant::get_entity_state::<homeassistant::EntityState>(
            &ha_config,
            &led_config.entity,
        ) {
            Ok(s) => {
                let in_zone = s.state == led_config.gps_zone;
                info!(
                    "Entity '{}' in zone '{}': {}",
                    led_config.entity, led_config.gps_zone, in_zone
                );
                // TODO: flicker
                led.set_level(in_zone.into()).unwrap();
            }
            Err(e) => {
                info!("Failed to fetch HA state: {}", e);
            }
        }
        Timer::after_secs(5).await;
    }
}
