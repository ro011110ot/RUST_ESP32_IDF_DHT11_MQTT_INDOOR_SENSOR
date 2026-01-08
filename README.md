ESP32 Environmental Monitoring Node (Rust)

An asynchronous IoT application built with **Rust** and **ESP-IDF** for monitoring environmental data. This project
reads temperature and humidity from a **DHT11** sensor and publishes the data via **MQTT**.

## 🚀 Features

- **Async WiFi Stack**: Reliable connection handling using `esp-idf-svc`'s asynchronous WiFi.
- **MQTT Publishing**: Transmits sensor data as JSON payloads for easy integration with Home Assistant or Node-RED.
- **Robust Sensor Logic**: Implementation of the DHT11 protocol with automatic retries and error logging.
- **Environment Aware**: Uses a `build.rs` script to inject sensitive credentials (SSID, MQTT) from a `.env` file at
  compile time.
- **Optimized for ESP32**: Tailored for the Xtensa architecture with customized build profiles.

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