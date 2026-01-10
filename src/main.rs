mod dht11;
mod mqtt;
mod wifi;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::mqtt::client::QoS; // Nutzt QoS wieder
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

    info!("Booting DHT11 Indoor Node (MariaDB Compatibility Mode)...");

    // 1. DHT11 Pin Initialization
    let pin4: AnyIOPin = peripherals.pins.gpio4.into();
    let mut dht_pin = PinDriver::input_output(pin4)?;
    dht_pin.set_pull(Pull::Up)?;
    dht_pin.set_high()?;

    info!("Waiting for DHT11 stabilization...");
    FreeRtos::delay_ms(2000);
    let mut dht_sensor = dht11::DhtSensor::new(dht_pin);

    // 2. WiFi Setup
    let _wifi = block_on(wifi::connect_wifi(
        peripherals.modem,
        sys_loop.clone(),
        timer_service.clone(),
        nvs,
    ))?;

    // 3. NTP Sync
    let _sntp = EspSntp::new_default()?;
    info!("Waiting for NTP sync...");
    while SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() < 1600000000 {
        FreeRtos::delay_ms(1000);
    }
    info!("Time synchronized.");

    // 4. MQTT Client Setup
    let mut mqtt_client = mqtt::create_mqtt_client()?;
    let mut last_processed_slot = 0;

    info!("Entering main loop (15min intervals)...");

    loop {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let current_secs = now.as_secs();

        // 15-Minuten Intervall (900s)
        if current_secs % 900 == 0 {
            let current_slot = current_secs / 900;

            if current_slot != last_processed_slot {
                let minute = (current_secs / 60) % 60;
                info!("Interval triggered (Minute {:02})!", minute);

                let mut measurement = None;

                // Wir versuchen die Messung mit etwas mehr Geduld
                for attempt in 1..=5 {
                    if let Some(data) = dht_sensor.read_data() {
                        measurement = Some(data);
                        break;
                    }
                    warn!("Read attempt {} failed, retrying in 3s...", attempt);
                    FreeRtos::delay_ms(3000);
                }

                if let Some((temp, hum)) = measurement {
                    info!("Measurement successful: {:.1}C, {:.1}%", temp, hum);
                    let base_topic = env!("MQTT_TOPIC");

                    let temp_payload = format!(r#"{{"id": "DHT11_Indoor", "Temp": {:.1}}}"#, temp);
                    let hum_payload =
                        format!(r#"{{"id": "DHT11_Indoor", "Humidity": {:.1}}}"#, hum);

                    // MQTT Publish Logik (Beseitigt die Warnungen)
                    if let Err(e) = mqtt_client.publish(
                        &format!("{}/Temp", base_topic),
                        QoS::AtLeastOnce,
                        false,
                        temp_payload.as_bytes(),
                    ) {
                        error!("Failed to publish temperature: {:?}", e);
                    }

                    if let Err(e) = mqtt_client.publish(
                        &format!("{}/Humidity", base_topic),
                        QoS::AtLeastOnce,
                        false,
                        hum_payload.as_bytes(),
                    ) {
                        error!("Failed to publish humidity: {:?}", e);
                    }

                    info!("Data sent to MariaDB.");
                } else {
                    error!("Failed to get valid data after 5 retries.");
                }

                last_processed_slot = current_slot;
            }
        }

        FreeRtos::delay_ms(1000);
    }
}
