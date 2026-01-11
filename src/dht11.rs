use dht11::Dht11;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{AnyIOPin, InputOutput, PinDriver};

/// Wrapper for the DHT11 sensor using a PinDriver.
pub struct DhtSensor<'a> {
    /// The actual DHT11 device driver.
    /// It owns the PinDriver to satisfy embedded-hal trait requirements.
    device: Dht11<PinDriver<'a, AnyIOPin, InputOutput>>,
}

impl<'a> DhtSensor<'a> {
    /// Creates a new DhtSensor instance.
    pub fn new(pin: PinDriver<'a, AnyIOPin, InputOutput>) -> Self {
        Self {
            device: Dht11::new(pin),
        }
    }

    /// Performs a measurement and returns (temperature, humidity).
    /// Returns None if the measurement fails (e.g., Timeout or Checksum error).
    pub fn read_data(&mut self) -> Option<(f32, f32)> {
        let mut delay = Ets;

        // Perform the measurement using the Ets (Espressif Timer Service) delay.
        match self.device.perform_measurement(&mut delay) {
            Ok(measurement) => {
                // For most DHT11 libraries, temperature and humidity are returned
                // as integer values. If your sensor requires decimal precision
                // (DHT22 style), you might need to divide by 10.0.
                let temp = measurement.temperature as f32;
                let hum = measurement.humidity as f32;

                Some((temp, hum))
            }
            Err(e) => {
                // Log the specific error (Timeout, Checksum, etc.) to the console.
                log::error!("DHT11 Read Error: {:?}", e);
                None
            }
        }
    }
}
