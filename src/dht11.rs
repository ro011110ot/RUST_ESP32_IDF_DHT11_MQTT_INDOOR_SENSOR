use dht11::Dht11;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{AnyIOPin, InputOutput, PinDriver};

/// Wrapper for the DHT11 sensor using a PinDriver.
pub struct DhtSensor<'a> {
    device: Dht11<PinDriver<'a, AnyIOPin, InputOutput>>,
}

impl<'a> DhtSensor<'a> {
    pub fn new(pin: PinDriver<'a, AnyIOPin, InputOutput>) -> Self {
        Self {
            device: Dht11::new(pin),
        }
    }

    /// Read temperature and humidity. Returns Some((temp, hum)) or None.
    pub fn read_data(&mut self) -> Option<(f32, f32)> {
        let mut delay = Ets;

        match self.device.perform_measurement(&mut delay) {
            Ok(measurement) => {
                // DHT11 values in your logs (e.g. 212) represent 21.2
                // Scaling by 10.0 to get the actual float values
                let temp = measurement.temperature as f32 / 10.0;
                let hum = measurement.humidity as f32 / 10.0;

                Some((temp, hum))
            }
            Err(e) => {
                log::error!("DHT11 Read Error: {:?}", e);
                None
            }
        }
    }
}
