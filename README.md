# 🌡️ ESP32 DHT11 Indoor Node (Rust)

An asynchronous IoT application for monitoring indoor climate using a **DHT11** sensor. This node is designed for
continuous operation (mains powered) and publishes data via **MQTT** in 15-minute intervals.

---

## 🚀 Features

- **Continuous Operation**: Optimized for mains power with persistent WiFi and MQTT connections.
- **Synchronized Measurement**: Transmits data exactly at **XX:00, XX:15, XX:30, and XX:45** using NTP synchronization.
- **Robust Sensor Logic**: Implementation with 5 retries and 2-second delays for reliable DHT11 readings.
- **MQTT QoS 1**: Guaranteed delivery with broker acknowledgment before the loop resets.
- **Environment Driven**: Credentials managed via `.env` file at compile time.

---

## 🛠 Hardware Setup

- **Microcontroller**: ESP32 (Xtensa).
- **Sensor**: DHT11.
- **Wiring**:
    - **VCC**: 3.3V / 5V
    - **GND**: Ground
    - **Data**: **GPIO 4**.

---

## ⚙️ Configuration

Create a `.env` file in the project root:

```env
WIFI_SSID="Your_SSID"
WIFI_PASS="Your_Password"
MQTT_BROKER="mqtt://your.broker.ip:1883"
MQTT_USER="your_user"
MQTT_PASS="your_password"
MQTT_TOPIC="Sensors/Indoor"
```

## 🔨 Build and Flash

Bash

```
cargo run
```

## 🔄 Execution Flow

Initialize: Sets up peripherals and English-language logging.

Network: Connects to WiFi and synchronizes time via NTP.

MQTT: Establishes a persistent connection to the broker.

Sync Loop: Checks the system time every second and triggers a measurement every 15 minutes.

Publish: Sends JSON data with QoS 1 and waits for server acknowledgment.

## 📝 License

MIT License