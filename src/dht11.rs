use dht11::Dht11;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{AnyIOPin, InputOutput, PinDriver};

pub struct DhtSensor<'a> {
    device: Dht11<PinDriver<'a, AnyIOPin, InputOutput>>,
}

impl<'a> DhtSensor<'a> {
    /// Creates a new instance of the DHT11 sensor
    pub fn new(pin: PinDriver<'a, AnyIOPin, InputOutput>) -> Self {
        Self {
            device: Dht11::new(pin),
        }
    }

    /// Performs a measurement and returns (temperature, humidity)
    pub fn read_data(&mut self) -> Option<(f32, f32)> {
        let mut delay = Ets;

        match self.device.perform_measurement(&mut delay) {
            Ok(measurement) => {
                // Convert raw sensor values to floating point numbers.
                // Note: Depending on the specific DHT11 crate version,
                // raw values might need to be divided by 10.0 to get the correct decimal value.
                let temp = measurement.temperature as f32 / 10.0;
                let hum = measurement.humidity as f32 / 10.0;

                Some((temp, hum))
            }
            Err(e) => {
                log::error!("DHT11 Error: {:?}", e);
                None
            }
        }
    }
}
