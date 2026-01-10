use dht11::Dht11;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{AnyIOPin, InputOutput, PinDriver};

pub struct DhtSensor<'a> {
    device: Dht11<PinDriver<'a, AnyIOPin, InputOutput>>,
}

impl<'a> DhtSensor<'a> {
    pub fn new(pin: PinDriver<'a, AnyIOPin, InputOutput>) -> Self {
        Self {
            device: Dht11::new(pin),
        }
    }

    pub fn read_data(&mut self) -> Option<(f32, f32)> {
        let mut delay = Ets;

        // Give the sensor a tiny bit of breath before starting the handshake
        esp_idf_svc::hal::delay::FreeRtos::delay_ms(10);

        match self.device.perform_measurement(&mut delay) {
            Ok(measurement) => {
                // DHT11 library usually returns 10x the value for integers
                // If your values were 215 instead of 21.5, the / 10.0 is correct.
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
