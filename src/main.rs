use config::FlickeringGpsLed;
use embassy_executor::{main, Spawner};
use embassy_time::Duration;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::{AnyOutputPin, Output, PinDriver},
        prelude::*,
    },
    nvs::EspDefaultNvsPartition,
};
use log::{error, info};
use std::sync::Arc;
use wifi::WithWifiTask;

mod homeassistant;
mod wifi;
mod flicker;

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
        // Config validation should mean we don't reuse GPIO pins.
        let pin = unsafe { AnyOutputPin::new(cfg.gpio_pin) };
        let task = FlickeringGpsLedTask {
            ha_config: config.home_assistant_config.clone(),
            led_config: cfg.clone(),
            led: PinDriver::output(pin).unwrap(),
        };
        spawner.must_spawn(flickering_gps_led_runner(wifi_status.clone(), task));
    }
}

struct FlickeringGpsLedTask<'a, 'r> {
    ha_config: config::HomeAssistantConfig,
    led_config: FlickeringGpsLed,
    led: PinDriver<'a, AnyOutputPin, Output>,
    flicker: flicker::FlickerSequence<'r, rand::rngs::ThreadRng>
}
impl<'a> WithWifiTask for FlickeringGpsLedTask<'a> {
    fn get_sleep_duration(&self) -> Duration {
        Duration::from_secs(5)
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        match homeassistant::get_entity_state::<homeassistant::EntityState>(
            &self.ha_config,
            &self.led_config.entity,
        ) {
            Ok(s) => {
                let in_zone = s.state == self.led_config.gps_zone;
                info!(
                    "Entity '{}' in zone '{}': {}",
                    self.led_config.entity, self.led_config.gps_zone, in_zone
                );
                // TODO: flicker
                self.led.set_level(in_zone.into()).unwrap();
            }
            Err(e) => {
                error!("Failed to fetch HA state: {}", e);
            }
        }
        Ok(())
    }
}
#[embassy_executor::task]
async fn flickering_gps_led_runner(
    wifi_status: Arc<wifi::WifiStatus>,
    mut task: FlickeringGpsLedTask<'static>,
) -> ! {
    task.loop_when_wifi(wifi_status).await
}
