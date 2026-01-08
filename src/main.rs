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

    info!("Booting DHT11 Indoor Node (Enhanced Recovery & Slot-Locking)...");

    // 1. Initial WiFi Setup
    let mut wifi = block_on(wifi::connect_wifi(
        peripherals,
        sys_loop.clone(),
        timer_service.clone(),
        nvs,
    ))?;

    // 2. NTP Sync (Required for interval logic)
    let _sntp = EspSntp::new_default()?;
    while SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() < 1736340000 {
        info!("Waiting for NTP sync...");
        FreeRtos::delay_ms(2000);
    }
    info!("Time synchronized.");

    // 3. Sensor & MQTT Initialization
    let dht_pin = PinDriver::input_output(unsafe { AnyIOPin::new(4) })?;
    let mut sensor = dht11::DhtSensor::new(dht_pin);
    let mut mqtt_client = mqtt::create_mqtt_client()?;

    let interval = 900; // 15 minutes
    let mut last_processed_slot = 0;
    let mut reconnect_counter = 0;

    info!("Entering main loop...");

    loop {
        // --- WIFI RECOVERY LOGIC ---
        match wifi.is_up() {
            Ok(connected) => {
                if !connected {
                    reconnect_counter += 1;
                    warn!("WiFi down (Attempt {}). Reconnecting...", reconnect_counter);

                    if reconnect_counter > 20 {
                        error!("WiFi recovery failed. Restarting chip...");
                        esp_idf_svc::hal::reset::restart();
                    } else if reconnect_counter % 5 == 0 {
                        info!("Cycling WiFi stack...");
                        let _ = wifi.stop();
                        FreeRtos::delay_ms(1000);
                        let _ = wifi.start();
                    }

                    let _ = wifi.connect();
                    FreeRtos::delay_ms(5000);
                    continue;
                } else if reconnect_counter > 0 {
                    info!("WiFi restored!");
                    reconnect_counter = 0;
                }
            }
            Err(e) => error!("WiFi health check error: {:?}", e),
        }

        // --- MEASUREMENT LOGIC WITH SLOT-LOCKING ---
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let current_slot = now - (now % interval);
        let seconds_past_slot = now % interval;

        // Check if we entered a NEW 15-minute slot
        if current_slot > last_processed_slot {
            // Only attempt measurement within the first 60 seconds of the slot
            if seconds_past_slot < 60 {
                info!("Interval reached! Starting DHT11 measurement cycle...");

                let mut measurement = None;
                for i in 1..=5 {
                    if let Some((temp, hum)) = sensor.read_data() {
                        info!("Measurement success: {:.1}C, {:.1}% RH", temp, hum);
                        measurement = Some((temp, hum));
                        break;
                    }
                    warn!("Read attempt {} failed, retrying in 2s...", i);
                    FreeRtos::delay_ms(2000);
                }

                if let Some((temp, hum)) = measurement {
                    let base_topic = env!("MQTT_TOPIC");
                    let temp_payload = format!(r#"{{"id": "DHT11_Indoor", "Temp": {:.1}}}"#, temp);
                    let hum_payload =
                        format!(r#"{{"id": "DHT11_Indoor", "Humidity": {:.1}}}"#, hum);

                    info!("Publishing to MQTT...");
                    if let Err(e) = mqtt_client.publish(
                        &format!("{}/Temp", base_topic),
                        QoS::AtLeastOnce,
                        false,
                        temp_payload.as_bytes(),
                    ) {
                        error!("MQTT Temp publish error: {:?}", e);
                    }
                    if let Err(e) = mqtt_client.publish(
                        &format!("{}/Humidity", base_topic),
                        QoS::AtLeastOnce,
                        false,
                        hum_payload.as_bytes(),
                    ) {
                        error!("MQTT Hum publish error: {:?}", e);
                    }

                    // Small delay to ensure MQTT buffers are cleared
                    FreeRtos::delay_ms(2000);
                } else {
                    error!("Failed to get valid sensor data in this interval.");
                }

                // CRITICAL: Mark this slot as processed to prevent re-entry 1s later
                last_processed_slot = current_slot;
            }
        }

        // Prevent CPU starvation (1Hz check rate)
        FreeRtos::delay_ms(1000);
    }
}
