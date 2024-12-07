#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::ToString;
use config::FlickeringGpsLed;
use embassy_executor::Spawner;
use embassy_time::Duration;
use embedded_storage::ReadStorage;
use esp_alloc::heap_allocator;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Output, OutputPin},
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_storage::FlashStorage;
use log::{error, info};
use wifi::WithWifiTask;

//mod homeassistant;
mod wifi;

const HEAP_MEMORY_SIZE: usize = 20 * 1024;

fn load_config_or_die() -> Result<config::Config, esp_storage::FlashStorageError> {
    let mut flash = FlashStorage::new();
    let config_partition_base_offset = 0x7000u32;
    let mut buffer = [0u8; 0x3000];
    flash.read(config_partition_base_offset, &mut buffer)?;

    Ok(config::Config::parse_or_panic(&buffer))
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let mut esp_config = esp_hal::Config::default();
    esp_config.cpu_clock = CpuClock::max();
    let peripherals = esp_hal::init(esp_config);

    heap_allocator!(HEAP_MEMORY_SIZE);

    let tg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(tg0.timer0);

    let config = load_config_or_die().unwrap();
    info!("Config: {:?}", config);

    let rng = Rng::new(peripherals.RNG);
    let wifi_status = wifi::init_and_spawn_tasks(
        spawner,
        tg0.timer1,
        rng,
        peripherals.WIFI,
        peripherals.RADIO_CLK,
        config.networks,
    );

    //for cfg in config.flickering_gps_leds {
    //    // Config validation should mean we don't reuse GPIO pins.
    //    let pin = unsafe { Output::new(cfg.gpio_pin) };
    //    let task = FlickeringGpsLedTask {
    //        ha_config: config.home_assistant_config.clone(),
    //        led_config: cfg.clone(),
    //        led: PinDriver::output(pin).unwrap(),
    //    };
    //    spawner.must_spawn(flickering_gps_led_runner(wifi_status.clone(), task));
    //}
}

//struct FlickeringGpsLedTask<'a> {
//    ha_config: config::HomeAssistantConfig,
//    led_config: FlickeringGpsLed,
//    led: PinDriver<'a, AnyOutputPin, Output>,
//}
//impl<'a> WithWifiTask for FlickeringGpsLedTask<'a> {
//    fn get_sleep_duration(&self) -> Duration {
//        Duration::from_secs(5)
//    }
//
//    async fn run(&mut self) -> anyhow::Result<()> {
//        match homeassistant::get_entity_state::<homeassistant::EntityState>(
//            &self.ha_config,
//            &self.led_config.entity,
//        ) {
//            Ok(s) => {
//                let in_zone = s.state == self.led_config.gps_zone;
//                info!(
//                    "Entity '{}' in zone '{}': {}",
//                    self.led_config.entity, self.led_config.gps_zone, in_zone
//                );
//                // TODO: flicker
//                self.led.set_level(in_zone.into()).unwrap();
//            }
//            Err(e) => {
//                error!("Failed to fetch HA state: {}", e);
//            }
//        }
//        Ok(())
//    }
//}
//#[embassy_executor::task]
//async fn flickering_gps_led_runner(
//    wifi_status: Arc<wifi::WifiStatus>,
//    mut task: FlickeringGpsLedTask<'static>,
//) -> ! {
//    task.loop_when_wifi(wifi_status).await
//}
//
