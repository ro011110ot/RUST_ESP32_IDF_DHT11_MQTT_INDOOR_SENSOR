use core::convert::TryInto;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::sys::{esp_wifi_set_ps, wifi_ps_type_t_WIFI_PS_NONE};
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use log::{error, info, warn};

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASS");
const MAX_RETRIES: u8 = 10;

/// Setup and connect to the Wi-Fi network.
pub async fn connect_wifi(
    modem: Modem,
    sys_loop: EspSystemEventLoop,
    timer_service: EspTaskTimerService,
    _nvs: EspDefaultNvsPartition,
) -> anyhow::Result<AsyncWifi<EspWifi<'static>>> {
    let esp_wifi = EspWifi::new(modem, sys_loop.clone(), None)?;
    let mut wifi = AsyncWifi::wrap(esp_wifi, sys_loop, timer_service)?;

    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    info!("Starting Wi-Fi...");
    wifi.start().await?;
    unsafe {
        esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_NONE);
    }
    info!("Wi-Fi Power Management disabled for stable sensor timing.");

    let mut attempts = 0;
    loop {
        attempts += 1;
        info!(
            "Connecting to SSID: {} (Attempt {}/{})...",
            SSID, attempts, MAX_RETRIES
        );

        match wifi.connect().await {
            Ok(_) => {
                info!("Waiting for network interface...");
                match wifi.wait_netif_up().await {
                    Ok(_) => {
                        info!("Wi-Fi connected successfully!");
                        return Ok(wifi);
                    }
                    Err(e) => warn!("Netif up failed: {:?}", e),
                }
            }
            Err(e) => warn!("Connection attempt failed: {:?}", e),
        }

        if attempts >= MAX_RETRIES {
            error!(
                "FATAL: Failed to connect after {} attempts. Restarting...",
                MAX_RETRIES
            );
            esp_idf_svc::hal::reset::restart();
        }

        FreeRtos::delay_ms(5000);
    }
}
