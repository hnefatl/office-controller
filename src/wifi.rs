use alloc::sync::Arc;
use alloc::vec::Vec;
use embassy_executor::Spawner;
use embassy_net::{DhcpConfig, StackResources};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use esp_hal::peripheral::Peripheral;
use esp_hal::peripherals::{RADIO_CLK, TIMG0, WIFI};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::Blocking;
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiError, WifiEvent, WifiStaDevice,
    WifiState,
};
use esp_wifi::{EspWifiController, EspWifiTimerSource};
use log::{error, info, warn};
use rand::RngCore;
use static_cell::StaticCell;

type Stack = embassy_net::Stack<esp_wifi::wifi::WifiDevice<'static, WifiStaDevice>>;

const NUM_SOCKETS: usize = 5;

fn parse_auth_method(a: config::AuthMethod) -> esp_wifi::wifi::AuthMethod {
    match a {
        config::AuthMethod::None => esp_wifi::wifi::AuthMethod::None,
        config::AuthMethod::WPA2Personal => esp_wifi::wifi::AuthMethod::WPA2Personal,
    }
}

pub fn init_and_spawn_tasks<T: EspWifiTimerSource>(
    spawner: Spawner,
    timer: impl Peripheral<P = T> + 'static,
    rng: Rng,
    wifi: WIFI,
    radio_clock_control: RADIO_CLK,
    networks: Vec<config::WifiNetwork>,
) -> Arc<WifiStatus> {
    let wifi_status = Arc::new(WifiStatus::new());

    let (stack, controller) = init(timer, rng, wifi, radio_clock_control);

    spawner.must_spawn(run_network(stack));
    spawner.must_spawn(repeatedly_connect_to_wifi(
        controller,
        wifi_status.clone(),
        networks,
    ));
    wifi_status
}

/// Initialises wifi for the chip. Must be called at most once.
fn init<T: EspWifiTimerSource>(
    timer: impl Peripheral<P = T> + 'static,
    mut rng: Rng,
    wifi: WIFI,
    radio_clock_control: RADIO_CLK,
) -> (&'static Stack, &'static mut WifiController<'static>) {
    // TODO: return a Result rather than unwrapping: the internal errors don't implement Error though :(

    static STACK_RESOURCES: StaticCell<StackResources<NUM_SOCKETS>> = StaticCell::new();
    let stack_resources = STACK_RESOURCES.init(StackResources::new());

    static ESP_WIFI_CONTROLLER: StaticCell<EspWifiController> = StaticCell::new();
    let esp_wifi_controller =
        ESP_WIFI_CONTROLLER.init(esp_wifi::init(timer, rng, radio_clock_control).unwrap());

    let (wifi_device, wifi_controller) =
        esp_wifi::wifi::new_with_mode(esp_wifi_controller, wifi, WifiStaDevice).unwrap();

    static WIFI_CONTROLLER: StaticCell<WifiController> = StaticCell::new();
    let wifi_controller = WIFI_CONTROLLER.init(wifi_controller);

    static STACK: StaticCell<Stack> = StaticCell::new();
    let stack = STACK.init(Stack::new(
        wifi_device,
        embassy_net::Config::dhcpv4(DhcpConfig::default()),
        stack_resources,
        rng.next_u64(),
    ));
    (stack, wifi_controller)
}

#[embassy_executor::task]
async fn run_network(stack: &'static Stack) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn repeatedly_connect_to_wifi(
    controller: &'static mut WifiController<'static>,
    status: Arc<WifiStatus>,
    networks: Vec<config::WifiNetwork>,
) -> ! {
    for network in networks.iter().cycle() {
        if let Err(e) = try_connect(controller, network).await {
            warn!("Failed to connect to wifi: {:?}", e);
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }

        info!("Connected, waiting until disconnect");
        status.set_connected(true).await;
        controller.wait_for_event(WifiEvent::StaDisconnected).await;
        status.set_connected(false).await;
        error!("Disconnected from wifi");
    }
    panic!("No network config provided")
}

async fn try_connect(
    controller: &mut WifiController<'_>,
    network: &config::WifiNetwork,
) -> core::result::Result<(), WifiError> {
    // Start fresh if already connected.
    if esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
        warn!("Already connected, disconnecting");
        controller.disconnect_async().await;
    }

    info!("Attempting connection to WiFi SSID '{}'", network.ssid);
    controller.set_configuration(&Configuration::Client(ClientConfiguration {
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
        auth_method: parse_auth_method(network.auth_method),
        ..Default::default()
    }))?;
    controller.connect_async().await?;
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
