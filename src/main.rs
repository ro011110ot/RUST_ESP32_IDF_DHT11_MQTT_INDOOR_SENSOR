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

    // Initialize peripherals and services
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let timer_service = EspTaskTimerService::new()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Base topic from .env (e.g., "Sensors/Indoor")
    const BASE_TOPIC: &str = env!("MQTT_TOPIC");
    // Deep sleep duration: 15 minutes (900 seconds)
    const SLEEP_TIME_SECONDS: u64 = 900;

    // Establish Wi-Fi connection
    let _wifi = block_on(wifi::connect_wifi(
        peripherals,
        sys_loop,
        timer_service,
        nvs,
    ))?;

    // DHT11 sensor connected to GPIO 4
    let dht_pin = PinDriver::input_output(unsafe { esp_idf_svc::hal::gpio::AnyIOPin::new(4) })?;
    let mut sensor = dht11::DhtSensor::new(dht_pin);

    // Initialize MQTT Client
    let mut mqtt_client = mqtt::create_mqtt_client()?;

    // Short stabilization delay after boot
    info!("System started. Waiting 2s for sensor stabilization...");
    FreeRtos::delay_ms(2000);

    let mut success = false;
    let mut retries = 3;

    // Measurement and publishing logic
    while retries > 0 && !success {
        if let Some((temp, hum)) = sensor.read_data() {
            let temp_topic = format!("{}/Temp", BASE_TOPIC);
            let hum_topic = format!("{}/Humidity", BASE_TOPIC);

            // Payload in JSON format: {"id": "DHT11", "Temp": 22.5}
            let temp_payload = format!(r#"{{"id": "DHT11", "Temp": {:.1}}}"#, temp);
            let hum_payload = format!(r#"{{"id": "DHT11", "Humidity": {:.1}}}"#, hum);

            info!("Publishing to MQTT: {} / {}", temp_topic, hum_topic);

            let _ =
                mqtt_client.publish(&temp_topic, QoS::AtMostOnce, false, temp_payload.as_bytes());
            let _ = mqtt_client.publish(&hum_topic, QoS::AtMostOnce, false, hum_payload.as_bytes());

            success = true;
            info!("Data sent successfully!");
        } else {
            retries -= 1;
            if retries > 0 {
                error!(
                    "DHT11 Error: Timeout. Retrying in 2s... ({} attempts left)",
                    retries
                );
                FreeRtos::delay_ms(2000);
            } else {
                error!("DHT11 Final Error: Sensor not responding.");
            }
        }
    }

    // Ensure MQTT messages are sent before entering sleep
    info!("Waiting for transmission to finish...");
    FreeRtos::delay_ms(2000);

    // Entering deep sleep mode
    info!("Entering deep sleep for {} seconds...", SLEEP_TIME_SECONDS);
    unsafe {
        esp_sleep_enable_timer_wakeup(SLEEP_TIME_SECONDS * 1_000_000);
        esp_deep_sleep_start();
    }

    // This code is unreachable due to deep sleep reset
    #[allow(unreachable_code)]
    Ok(())
}
