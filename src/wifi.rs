use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::Modem,
    nvs::EspDefaultNvsPartition,
    sys::EspError,
    timer::EspTaskTimerService,
    wifi::{self, AsyncWifi, ClientConfiguration, EspWifi},
};
use log::{error, info, warn};
use std::sync::Arc;

#[embassy_executor::task]
pub async fn repeatedly_connect_to_wifi(
    modem: Modem,
    sysloop: EspSystemEventLoop,
    nvs: Option<EspDefaultNvsPartition>,
    status: Arc<WifiStatus>,
    networks: Vec<config::WifiNetwork>,
) -> ! {
    let esp_wifi = EspWifi::new(modem, sysloop.clone(), nvs).unwrap();
    let timer = EspTaskTimerService::new().unwrap();
    let mut wifi = AsyncWifi::wrap(esp_wifi, sysloop.clone(), timer).unwrap();

    wifi.start().await.unwrap();
    for network in networks.iter().cycle() {
        if let Err(e) = try_connect(&mut wifi, network).await {
            warn!("Failed to connect to wifi: {}", e);
            Timer::after(Duration::from_secs(1)).await;
            continue;
        }

        info!("Connected, waiting until disconnect");
        status.set_connected(true).await;
        wifi.wifi_wait(|w| w.is_connected(), None).await.unwrap();
        status.set_connected(false).await;
        error!("Disconnected from wifi");
    }
    panic!("No network config provided")
}

async fn try_connect(
    wifi: &mut AsyncWifi<EspWifi<'_>>,
    network: &config::WifiNetwork,
) -> std::result::Result<(), EspError> {
    // Start fresh if already connected.
    if wifi.is_connected()? {
        warn!("Already connected, disconnecting");
        let _ = wifi.disconnect().await;
    }

    info!("Attempting connection to WiFi SSID '{}'", network.ssid);
    wifi.set_configuration(&wifi::Configuration::Client(ClientConfiguration {
        ssid: network
            .ssid
            .parse()
            .expect("Could not parse the given SSID into WiFi config"),
        // This unfortunately can't be zeroized after use since it's owned by the configuration.
        password: network
            .password
            .insecure()
            .parse()
            .expect("Could not parse the given password into WiFi config"),
        channel: None, // Autodiscover the channel
        auth_method: network.auth_method,
        ..Default::default()
    }))?;
    wifi.connect().await?;
    wifi.wait_netif_up().await?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {:?}", ip_info);
    Ok(())
}

pub trait WithWifiTask {
    fn get_sleep_duration(&self) -> Duration;
    async fn run(&mut self) -> anyhow::Result<()>;

    async fn loop_when_wifi(&mut self, wifi_status: Arc<WifiStatus>) -> ! {
        loop {
            wifi_status.wait_until_connected().await;
            self.run().await.unwrap();
            Timer::after(self.get_sleep_duration()).await;
        }
    }
}

pub struct WifiStatus {
    connected: Mutex<CriticalSectionRawMutex, bool>,
}
impl WifiStatus {
    pub fn new() -> Self {
        WifiStatus {
            connected: Mutex::new(false),
        }
    }

    /// "Best-effort" wait: blocks until connected has been set at least once, but doesn't guarantee
    /// that it's still set at any point afterwards.
    /// Intended to act as a soft block of "if we're clearly disconnected, don't do anything until
    /// we're connected again".
    async fn wait_until_connected(&self) {
        while !*self.connected.lock().await {
            // Polling is ugly but simple. The embassy sync primitives require knowing at compile-time
            // how many tasks can wait at once, and that's more annoying to calculate and pass around
            // in a clean manner than it is to just spinlock.
            Timer::after_secs(1).await;
        }
    }

    async fn set_connected(&self, connected: bool) {
        *self.connected.lock().await = connected;
    }
}
