use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{modem::WifiModemPeripheral, peripheral::Peripheral},
    nvs::EspDefaultNvsPartition,
    sys::EspError,
    wifi::{self, BlockingWifi, ClientConfiguration, EspWifi},
};

use log::info;

pub struct Wifi<'d> {
    wifi: BlockingWifi<EspWifi<'d>>,
}
impl Wifi<'_> {
    pub fn new<'d, M>(
        modem: impl Peripheral<P = M> + 'd,
        sysloop: EspSystemEventLoop,
        nvs: Option<EspDefaultNvsPartition>,
    ) -> Result<Wifi<'d>, EspError>
    where
        M: WifiModemPeripheral,
    {
        let esp_wifi = EspWifi::new(modem, sysloop.clone(), nvs)?;
        let wifi = BlockingWifi::wrap(esp_wifi, sysloop.clone())?;
        Ok(Wifi { wifi })
    }

    pub fn start(&mut self) -> Result<(), EspError> {
        self.wifi.start()
    }

    pub fn try_connect(
        &mut self,
        network: &config::WifiNetwork,
    ) -> std::result::Result<(), EspError> {
        // Start fresh if already connected.
        if self.wifi.is_connected()? {
            info!("Already connected, disconnecting");
            let _ = self.wifi.disconnect();
        }

        info!("Attempting connection to WiFi SSID '{}'", network.ssid);
        self.wifi
            .set_configuration(&wifi::Configuration::Client(ClientConfiguration {
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
        self.wifi.connect()?;
        self.wifi.wait_netif_up()?;
        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;
        info!("Wifi DHCP info: {:?}", ip_info);
        Ok(())
    }
}
