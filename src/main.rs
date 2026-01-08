mod dht11;
mod mqtt;
mod wifi;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::mqtt::client::QoS;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition};
use esp_idf_sys::{esp_deep_sleep_start, esp_sleep_enable_timer_wakeup};
use log::{error, info};

fn main() -> anyhow::Result<()> {
    // Required for the ESP-IDF framework
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Initialize peripherals
    let peripherals = Peripherals::take()?;

    // Configuration constants
    const BASE_TOPIC: &str = env!("MQTT_TOPIC");
    const SLEEP_TIME_SECONDS: u64 = 900; // 15 minutes

    // 1. Initialize DHT11 sensor first
    let dht_pin = PinDriver::input_output(unsafe { esp_idf_svc::hal::gpio::AnyIOPin::new(4) })?;
    let mut sensor = dht11::DhtSensor::new(dht_pin);

    // 2. Give the sensor time to stabilize after boot/wake-up
    info!("Wake up detected. Waiting 10s for sensor stabilization...");
    FreeRtos::delay_ms(10000);

    let mut measurement: Option<(f32, f32)> = None;
    let mut retries = 5;

    // 3. Attempt to read from sensor before connecting to network (saves power)
    while retries > 0 {
        if let Some(data) = sensor.read_data() {
            measurement = Some(data);
            info!("Sensor read successful!");
            break;
        }
        retries -= 1;
        if retries > 0 {
            error!(
                "DHT11 timeout. Retrying in 2s... ({} attempts left)",
                retries
            );
            FreeRtos::delay_ms(2000);
        }
    }

    // 4. Only if we have valid data, we proceed with network operations
    if let Some((temp, hum)) = measurement {
        info!("Data acquired: {:.1}°C, {:.1}% humidity", temp, hum);

        // Initialize system services for networking
        let sys_loop = EspSystemEventLoop::take()?;
        let timer_service = EspTaskTimerService::new()?;
        let nvs = EspDefaultNvsPartition::take()?;

        // Connect to WiFi
        info!("Starting WiFi connection...");
        let _wifi = block_on(wifi::connect_wifi(
            peripherals,
            sys_loop,
            timer_service,
            nvs,
        ))?;

        // Initialize and connect MQTT client
        info!("Connecting to MQTT broker...");
        let mut mqtt_client = mqtt::create_mqtt_client()?;

        let temp_topic = format!("{}/Temp", BASE_TOPIC);
        let hum_topic = format!("{}/Humidity", BASE_TOPIC);

        // Prepare JSON payloads
        let temp_payload = format!(r#"{{"id": "DHT11", "Temp": {:.1}}}"#, temp);
        let hum_payload = format!(r#"{{"id": "DHT11", "Humidity": {:.1}}}"#, hum);

        info!("Publishing to MQTT: {} / {}", temp_topic, hum_topic);

        // Publish data
        let _ = mqtt_client.publish(&temp_topic, QoS::AtMostOnce, false, temp_payload.as_bytes());
        let _ = mqtt_client.publish(&hum_topic, QoS::AtMostOnce, false, hum_payload.as_bytes());

        // Wait a short moment to ensure transmission is handled by the network stack
        info!("Waiting for transmission to finish...");
        FreeRtos::delay_ms(2000);
    } else {
        error!("Could not read from sensor after all retries. Skipping transmission.");
    }

    // 5. Enter Deep Sleep (always happens, regardless of success)
    info!("Entering deep sleep for {} seconds...", SLEEP_TIME_SECONDS);
    unsafe {
        esp_sleep_enable_timer_wakeup(SLEEP_TIME_SECONDS * 1_000_000);
        esp_deep_sleep_start();
    }

    #[allow(unreachable_code)]
    Ok(())
}
