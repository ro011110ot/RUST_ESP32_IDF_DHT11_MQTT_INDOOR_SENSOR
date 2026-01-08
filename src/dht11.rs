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
        match self.device.perform_measurement(&mut delay) {
            Ok(measurement) => {
                // Wenn 241.0 geloggt wird, war der Rohwert 2410 (da 2410 / 10.0 = 241.0)
                // Das bedeutet, wir müssen durch 10.0 teilen, wenn wir Ganzzahlen wollen,
                // ODER dein Crate liefert bereits Zehntel und du teilst doppelt.

                // Versuch diese Kalibrierung:
                let temp = measurement.temperature as f32 / 10.0;
                let hum = measurement.humidity as f32 / 10.0;

                // Wenn die Werte dann immer noch zu hoch sind (z.B. 241),
                // entferne das "/ 10.0" komplett oder teile durch 100.0.
                Some((temp, hum))
            }
            Err(e) => {
                log::error!("DHT11 Fehler: {:?}", e);
                None
            }
        }
    }
}
