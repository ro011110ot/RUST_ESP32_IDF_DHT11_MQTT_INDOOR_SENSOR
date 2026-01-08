# ESP32 read DHT11 and send it via MQTT

An asynchronous IoT application built with **Rust** and **ESP-IDF** for monitoring environmental data. This project
reads temperature and humidity from a **DHT11** sensor and publishes the data via **MQTT**.

## 🚀 Features

- **Async Wi-Fi Stack**: Reliable connection handling using `esp-idf-svc`'s asynchronous Wi-Fi.
- **MQTT Publishing**: Transmits sensor data as JSON payloads for easy integration with Home Assistant or Node-RED.
- **Robust Sensor Logic**: Implementation of the DHT11 protocol with automatic retries and error logging.
- **Environment Aware**: Uses a `build.rs` script to inject sensitive credentials (SSID, MQTT) from a `.env` file at
  compile time.
- **Optimized for ESP32**: Tailored for the Xtensa architecture with customized build profiles.

### Power Management: Deep Sleep

To maximize battery life, this sensor is designed to operate in a low-power cycle:

1. **Wake-up:** The ESP32 boots and connects to Wi-Fi.
2. **Measurement:** The system reads data from the DHT11 sensor (with up to 3 retries).
3. **Transmission:** Data is published to the MQTT broker in JSON format.
4. **Deep Sleep:** The device enters deep sleep mode for **15 minutes**.
5. **Reset:** After the timer expires, the ESP32 performs a full restart and repeats the cycle.

### Energy Consumption

During deep sleep, the ESP32 consumes only a few microamps, making it suitable for long-term battery operation. Note
that all memory is cleared during sleep; the application starts fresh from the `main()` function upon waking up.

## 🛠 Hardware Setup

- **Microcontroller**: ESP32 (Xtensa architecture).
- **Sensor**: DHT11.
- **Wiring**:
    - **VCC**: 3.3V / 5V
    - **GND**: Ground
    - **Data Pin**: Connected to **GPIO 4** (as configured in `main.rs`).

## 📋 Prerequisites

Ensure your host system (e.g., Manjaro/Debian) has the following installed:

1. **ESP Toolchain**:
   ```bash
   espup install
   source $HOME/export-esp.sh
   ```
2. **Flash Utility**:
   ```bash
   cargo install espflash
   ```
3. **Build Dependencies**:
   ```bash
   # For Manjaro/Arch
   sudo pacman -S cmake ninja git
   ```

## ⚙️ Configuration

1. Create a `.env` file in the project root:
   ```env
   WIFI_SSID="Your_SSID"
   WIFI_PASS="Your_Password"
   MQTT_BROKER="mqtt://your.broker.ip:1883"
   MQTT_USER="your_user"
   MQTT_PASS="your_password"
   MQTT_TOPIC="Sensors/Indoor"
   ```
2. The `build.rs` will automatically pick up these variables.

## 🔨 Build and Flash

To build and flash the project to your ESP32:

```bash
# Flash and start the monitor
cargo run
🔍 Architecture
main.rs: Entry point, initializes peripherals and the main loop.

wifi.rs: Async WiFi connection logic.

mqtt.rs: MQTT client setup and event handling.

dht11.rs: Custom wrapper for the DHT11 sensor measurement.
```

📝 License
MIT License