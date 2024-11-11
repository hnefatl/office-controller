use anyhow::Result;
use embassy_executor::{main, Spawner};
use embassy_time::Timer;
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::prelude::*, nvs::EspDefaultNvsPartition};
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
    spawner.must_spawn(homeassistant_loop(
        wifi_status.clone(),
        config.home_assistant_config,
    ));
}

#[embassy_executor::task]
async fn homeassistant_loop(
    wifi_status: Arc<wifi::WifiStatus>,
    config: config::HomeAssistantConfig,
) -> ! {
    loop {
        wifi_status.wait_until_connected().await;

        match homeassistant::StateSnapshot::get(&config) {
            Ok(s) => match update(&config, s) {
                Ok(_) => {}
                Err(e) => {
                    info!("Failed to run update: {}", e);
                }
            },
            Err(e) => {
                info!("Failed to fetch HA state: {}", e);
            }
        }
        Timer::after_secs(5).await;
    }
}

fn update(
    config: &config::HomeAssistantConfig,
    state_snapshot: homeassistant::StateSnapshot,
) -> Result<()> {
    if state_snapshot.person_location.state == config.person_entity {
        info!("update pin");
    } else {
        info!(
            "unknown location {}, no-op",
            state_snapshot.person_location.state
        );
    }
    Ok(())
}
