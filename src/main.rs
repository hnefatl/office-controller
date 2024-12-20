use config::FlickeringGpsLed;
use embassy_executor::{main, Spawner};
use embassy_time::Duration;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::{AnyOutputPin, Output, PinDriver},
        prelude::*,
    },
    nvs::{EspCustomNvs, EspDefaultNvsPartition, EspNvsPartition, NvsCustom},
};
use log::{error, info};
use std::sync::Arc;
use wifi::WithWifiTask;

mod homeassistant;
mod wifi;

// Disabled until I've got NVS encryption configured, don't want to leak WiFi keys via flash.
const USE_PERSISTENT_WIFI_STORAGE: bool = false;

fn load_config_or_die() -> Result<config::Config, esp_idf_svc::sys::EspError> {
    // The namespace and config key name is from `config_partition.csv`. The partition name is from `partitions.csv`.
    // The config itself is loaded from `deployment_config.toml`.
    let config_partition = EspNvsPartition::<NvsCustom>::take("config")?;
    let config_nvs = EspCustomNvs::new(config_partition, "config", false)?;
    let config_len = config_nvs.str_len("config")?.unwrap();
    let mut config_buffer = vec![0u8; config_len];
    let config_text = config_nvs.get_str("config", &mut config_buffer)?.unwrap();
    Ok(config::Config::parse_or_panic(config_text))
}

fn log_stack_watermark(name: &str) {
    // This is the least remaining stack space seen during the runtime of the task.
    let watermark = unsafe { esp_idf_svc::sys::uxTaskGetStackHighWaterMark2(std::ptr::null_mut()) };
    info!("Stack watermark ({}): {}", name, watermark);
}

#[main]
async fn main(spawner: Spawner) {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log_stack_watermark("main");

    let config = load_config_or_die().unwrap();
    info!("Config: {:?}", config);

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

struct FlickeringGpsLedTask<'a> {
    ha_config: config::HomeAssistantConfig,
    led_config: FlickeringGpsLed,
    led: PinDriver<'a, AnyOutputPin, Output>,
}
impl<'a> WithWifiTask for FlickeringGpsLedTask<'a> {
    fn get_sleep_duration(&self) -> Duration {
        Duration::from_secs(5)
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        log_stack_watermark("FlickeringGpsLedTask::run");
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
