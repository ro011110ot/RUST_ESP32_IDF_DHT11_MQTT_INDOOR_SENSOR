use core::convert::TryInto;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use log::info;

// Fetch credentials from environment variables at compile time
const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");

pub async fn connect_wifi(
    peripherals: Peripherals,
    sys_loop: EspSystemEventLoop,
    timer_service: EspTaskTimerService,
    nvs: EspDefaultNvsPartition,
) -> anyhow::Result<AsyncWifi<EspWifi<'static>>> {
    let mut wifi = AsyncWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
        timer_service,
    )?;

    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    info!("Starting WiFi...");
    wifi.start().await?;

    info!("Connecting to {}...", SSID);
    wifi.connect().await?;

    info!("Waiting for network interface...");
    wifi.wait_netif_up().await?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!("WiFi connected. IP info: {:?}", ip_info);

    Ok(wifi)
}
