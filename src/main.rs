mod dht11;
mod mqtt;
mod wifi;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::mqtt::client::QoS;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, sntp::EspSntp};
use log::{error, info, warn};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let timer_service = EspTaskTimerService::new()?;
    let nvs = EspDefaultNvsPartition::take()?;

    info!("Booting DHT11 Node with Auto-Reconnect...");

    // 1. Initial WiFi & NTP Sync
    let mut wifi = block_on(wifi::connect_wifi(
        peripherals,
        sys_loop.clone(),
        timer_service.clone(),
        nvs,
    ))?;

    let _sntp = EspSntp::new_default()?;
    while SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() < 1736340000 {
        info!("Waiting for NTP sync...");
        FreeRtos::delay_ms(2000);
    }
    info!("Time synchronized.");

    let dht_pin = PinDriver::input_output(unsafe { AnyIOPin::new(4) })?;
    let mut sensor = dht11::DhtSensor::new(dht_pin);

    // Initial MQTT setup
    let mut mqtt_client = mqtt::create_mqtt_client()?;

    let interval = 900;
    let mut last_processed_slot = 0;

    loop {
        // --- CHECK WIFI STATUS ---
        match wifi.is_up() {
            Ok(connected) => {
                if !connected {
                    warn!("WiFi connection lost! Attempting to reconnect...");
                    let _ = wifi.connect(); // Non-blocking connect attempt
                    FreeRtos::delay_ms(5000);
                    continue; // Skip this loop iteration
                }
            }
            Err(e) => error!("WiFi status check failed: {:?}", e),
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let current_slot = now - (now % interval);
        let seconds_past_slot = now % interval;

        if current_slot > last_processed_slot && seconds_past_slot < 60 {
            info!("Interval reached! Checking MQTT and measuring...");

            // If MQTT is disconnected, the client usually tries to reconnect internally,
            // but we ensure a healthy WiFi first.

            let mut measurement = None;
            for _i in 1..=5 {
                if let Some((temp, hum)) = sensor.read_data() {
                    measurement = Some((temp, hum));
                    break;
                }
                FreeRtos::delay_ms(2000);
            }

            if let Some((temp, hum)) = measurement {
                let base_topic = env!("MQTT_TOPIC");
                let temp_payload = format!(r#"{{"id": "DHT11_Indoor", "Temp": {:.1}}}"#, temp);
                let hum_payload = format!(r#"{{"id": "DHT11_Indoor", "Humidity": {:.1}}}"#, hum);

                // Publish with confirmation
                if let Err(e) = mqtt_client.publish(
                    &format!("{}/Temp", base_topic),
                    QoS::AtLeastOnce,
                    false,
                    temp_payload.as_bytes(),
                ) {
                    error!("Failed to publish Temperature: {:?}", e);
                }
                if let Err(e) = mqtt_client.publish(
                    &format!("{}/Humidity", base_topic),
                    QoS::AtLeastOnce,
                    false,
                    hum_payload.as_bytes(),
                ) {
                    error!("Failed to publish Humidity: {:?}", e);
                }

                FreeRtos::delay_ms(5000);
            }
            last_processed_slot = current_slot;
        }

        FreeRtos::delay_ms(1000);
    }
}
