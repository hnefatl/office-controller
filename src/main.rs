#![no_main]

use anyhow::{bail, Result};
use config::HomeAssistantConfig;
use embedded_svc::http::{client::Client, Method};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{delay::Delay, prelude::Peripherals},
    http::client::{Configuration, EspHttpConnection},
    sys::EspError,
    wifi::{self, BlockingWifi, ClientConfiguration, EspWifi},
};
use log::info;

#[no_mangle]
fn main() -> Result<()> {
    let config = config::Config::load_or_panic();

    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;

    // Connect to the Wi-Fi network
    let mut esp_wifi = EspWifi::new(peripherals.modem, sysloop.clone(), None)?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop.clone())?;
    info!("Starting wifi");
    wifi.start()?;

    main_loop(config, &mut wifi);
}

fn main_loop(config: config::Config, wifi: &mut BlockingWifi<&mut EspWifi<'_>>) -> ! {
    let delay = Delay::new_default();
    for network in config.networks.iter().cycle() {
        if let Err(e) = try_connect_wifi(wifi, network) {
            info!("Failed to connect to wifi: {}", e);
            delay.delay_ms(1000);
            continue;
        }

        loop {
            match get_location(&config.home_assistant_config) {
                Ok(s) => info!("{}", s.state),
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

fn try_connect_wifi(
    wifi: &mut BlockingWifi<&mut EspWifi<'_>>,
    network: &config::WifiNetwork,
) -> std::result::Result<(), EspError> {
    // Start fresh if already connected.
    if wifi.is_connected()? {
        info!("Already connected, disconnecting");
        let _ = wifi.disconnect();
    }

    info!("Attempting connection to WiFi SSID '{}'", network.ssid);
    wifi.set_configuration(&wifi::Configuration::Client(ClientConfiguration {
        ssid: network
            .ssid
            .parse()
            .expect("Could not parse the given SSID into WiFi config"),
        password: network
            .password
            .parse()
            .expect("Could not parse the given password into WiFi config"),
        channel: None, // Autodiscover the channel
        auth_method: network.auth_method,
        ..Default::default()
    }))?;
    wifi.connect()?;
    wifi.wait_netif_up()?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {:?}", ip_info);
    Ok(())
}

#[derive(serde::Deserialize, Debug)]
struct HAPersonStateResponse {
    state: String,
}

fn get_location(ha_config: &HomeAssistantConfig) -> Result<HAPersonStateResponse> {
    let connection = EspHttpConnection::new(&Configuration {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut client = Client::wrap(connection);

    let auth_header = format!("Bearer {}", ha_config.access_token);
    let headers = [
        ("content-type", "application/json"),
        ("Authorization", &auth_header),
    ];
    let request_url = format!(
        "{}/api/states/{}",
        &ha_config.base_url, ha_config.person_entity
    );
    info!("Connecting to {}", request_url);
    let request = client.request(Method::Get, &request_url, &headers)?;
    let response = request.submit()?;

    if !(200..=299).contains(&response.status()) {
        bail!(
            "HTTP request failed, error code {}: {}",
            response.status(),
            response.status_message().unwrap_or_default()
        );
    }

    let response = serde_json::from_reader(embedded_io_adapters::std::ToStd::new(response))?;

    Ok(response)
}
