mod dht11;
mod mqtt;
mod wifi;

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::reset::WakeupReason;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::mqtt::client::QoS;
use esp_idf_svc::sys::{esp_deep_sleep_start, esp_sleep_enable_timer_wakeup};
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, sntp::EspSntp};
use log::{error, info, warn};

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let timer_service = EspTaskTimerService::new()?;
    let nvs = EspDefaultNvsPartition::take()?;

    info!("Booting DHT11 Node... Reason: {:?}", WakeupReason::get());

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

    // Wait for NTP sync to provide valid time
    FreeRtos::delay_ms(5000);

    // 2. Interval Calculation (00, 15, 30, 45)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let interval = 15 * 60; // 900 seconds
    let seconds_past_last_interval = now % interval;

    // If not within the first 30 seconds of an interval, and it's a cold boot,
    // sync by sleeping until the next slot.
    if seconds_past_last_interval > 30 && WakeupReason::get() == WakeupReason::Unknown {
        let sleep_until_next = interval - seconds_past_last_interval;
        info!(
            "Not at interval. Syncing sleep: {}s remaining.",
            sleep_until_next
        );
        enter_deep_sleep(sleep_until_next);
        return Ok(());
    }

    // 3. Sensor Measurement
    let dht_pin = PinDriver::input_output(unsafe { AnyIOPin::new(4) })?;
    let mut sensor = dht11::DhtSensor::new(dht_pin);
    FreeRtos::delay_ms(2000); // DHT11 needs time

    let mut measurement = None;
    for i in 1..=5 {
        if let Some((temp, hum)) = sensor.read_data() {
            info!(
                "Read success on attempt {}: {:.1}C, {:.1}% RH",
                i, temp, hum
            );
            measurement = Some((temp, hum));
            break;
        } else {
            warn!("Read attempt {} failed.", i);
            FreeRtos::delay_ms(2000);
        }
    }

    // 4. MQTT Transmission
    if let Some((temp, hum)) = measurement {
        info!("Connecting to MQTT broker...");
        let mut mqtt_client = mqtt::create_mqtt_client()?;
        FreeRtos::delay_ms(2000);

        let base_topic = env!("MQTT_TOPIC");
        let temp_payload = format!(r#"{{"id": "DHT11_Indoor", "Temp": {:.1}}}"#, temp);
        let hum_payload = format!(r#"{{"id": "DHT11_Indoor", "Humidity": {:.1}}}"#, hum);

        info!("Publishing data to {}...", base_topic);
        let _ = mqtt_client.publish(
            &format!("{}/Temp", base_topic),
            QoS::AtMostOnce,
            false,
            temp_payload.as_bytes(),
        );
        let _ = mqtt_client.publish(
            &format!("{}/Humidity", base_topic),
            QoS::AtMostOnce,
            false,
            hum_payload.as_bytes(),
        );

        FreeRtos::delay_ms(2000);
        info!("Transmission finished.");
    } else {
        error!("Could not read from DHT11 after retries.");
    }

    // 5. Calculate sleep time until the NEXT exact 15m slot
    let now_after_work = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let sleep_secs = interval - (now_after_work % interval);

    info!("Work done. Next interval in {}s.", sleep_secs);
    enter_deep_sleep(sleep_secs);

    Ok(())
}

fn enter_deep_sleep(secs: u64) {
    unsafe {
        esp_sleep_enable_timer_wakeup(secs * 1_000_000);
        esp_deep_sleep_start();
    }
}
