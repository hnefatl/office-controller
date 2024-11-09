#![no_main]

use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::Delay, prelude::Peripherals},
};
use log::info;
use wifi::Wifi;

mod homeassistant;
mod wifi;

#[no_mangle]
fn main() -> Result<()> {
    let config = config::Config::load_or_panic();

    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    let mut wifi = wifi::Wifi::new(peripherals.modem, sysloop.clone())?;
    info!("Starting wifi");
    wifi.start()?;

    main_loop(config, &mut wifi);
}

fn main_loop(config: config::Config, wifi: &mut Wifi) -> ! {
    let delay = Delay::new_default();

    for network in config.networks.iter().cycle() {
        if let Err(e) = wifi.try_connect(network) {
            info!("Failed to connect to wifi: {}", e);
            delay.delay_ms(1000);
            continue;
        }

        loop {
            match homeassistant::StateSnapshot::get(&config.home_assistant_config) {
                Ok(s) => match update(&config, s) {
                    Ok(_) => {}
                    Err(e) => {
                        info!("Failed to run update: {}", e);
                        break;
                    }
                },
                Err(e) => {
                    info!("Failed to fetch HA state: {}", e);
                    break;
                }
            }
            delay.delay_ms(5000);
        }
    }
    panic!("No network config provided")
}

fn update(config: &config::Config, state_snapshot: homeassistant::StateSnapshot) -> Result<()> {
    if state_snapshot.person_location.state == config.home_assistant_config.person_entity {
        info!("update pin");
    } else {
        info!(
            "unknown location {}, no-op",
            state_snapshot.person_location.state
        );
    }
    Ok(())
}
