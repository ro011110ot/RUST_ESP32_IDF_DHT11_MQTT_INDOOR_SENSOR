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

    info!("Booting DHT11 Node in Continuous Mode...");

    // 1. WiFi & NTP Sync
    info!("Initializing WiFi...");
    let _wifi = block_on(wifi::connect_wifi(
        peripherals,
        sys_loop,
        timer_service,
        nvs,
    ))?;

    info!("Syncing time via NTP...");
    let _sntp = EspSntp::new_default()?;

    // Wait for valid NTP time (threshold: Jan 2026)
    while SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() < 1736340000 {
        info!("Waiting for NTP sync...");
        FreeRtos::delay_ms(2000);
    }
    info!("Time synchronized.");

    // Initialize Sensor
    let dht_pin = PinDriver::input_output(unsafe { AnyIOPin::new(4) })?;
    let mut sensor = dht11::DhtSensor::new(dht_pin);

    // Persistent MQTT Connection
    let mut mqtt_client = mqtt::create_mqtt_client()?;

    let interval = 900; // 15 minutes
    let mut last_processed_slot = 0;

    info!("Entering main loop...");

    loop {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let current_slot = now - (now % interval);
        let seconds_past_slot = now % interval;

        // Trigger measurement in the first 60s of the 15m interval
        if current_slot > last_processed_slot && seconds_past_slot < 60 {
            info!("Interval reached! Starting DHT11 measurement...");

            let mut measurement = None;
            for i in 1..=5 {
                if let Some((temp, hum)) = sensor.read_data() {
                    info!(
                        "Read success on attempt {}: {:.1}C, {:.1}% RH",
                        i, temp, hum
                    );
                    measurement = Some((temp, hum));
                    break;
                }
                warn!("Read attempt {} failed, retrying...", i);
                FreeRtos::delay_ms(2000);
            }

            if let Some((temp, hum)) = measurement {
                let base_topic = env!("MQTT_TOPIC");
                let temp_payload = format!(r#"{{"id": "DHT11_Indoor", "Temp": {:.1}}}"#, temp);
                let hum_payload = format!(r#"{{"id": "DHT11_Indoor", "Humidity": {:.1}}}"#, hum);

                info!("Publishing data with QoS 1...");
                let _ = mqtt_client.publish(
                    &format!("{}/Temp", base_topic),
                    QoS::AtLeastOnce,
                    false,
                    temp_payload.as_bytes(),
                );
                let _ = mqtt_client.publish(
                    &format!("{}/Humidity", base_topic),
                    QoS::AtLeastOnce,
                    false,
                    hum_payload.as_bytes(),
                );

                // Allow network stack time to process ACKs
                FreeRtos::delay_ms(5000);
                info!("Transmission successful.");
            } else {
                error!("Failed to read from DHT11 after 5 attempts.");
            }

            last_processed_slot = current_slot;
        }

        FreeRtos::delay_ms(1000);
    }
}
