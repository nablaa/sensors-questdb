mod common;

use crate::common::Measurement;
use crate::common::send_measurement;
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Adapter;
use btleplug::platform::Manager;
use std::time::Duration;

const BLUETOOTH_SLEEP: Duration = Duration::from_secs(10);
const SENSOR_LOCATION: &str = "dummy";
const SENSOR_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x0000181a00001000800000805f9b34fb);

#[tokio::main]
async fn main() {
    let bluetooth_measurement = listen_bluetooth_measurement()
        .await
        .expect("Reading bluetooth measurement should succeed");
    send_measurement(&bluetooth_measurement).expect("Sending bluetooth measurement should succeed");
    println!("All done.");
}

async fn listen_bluetooth_measurement() -> Result<Measurement, Box<dyn std::error::Error>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.first().expect("Should find bluetooth adapeter");

    println!("Starting BLE scan ...");
    adapter.start_scan(ScanFilter::default()).await?;

    // Give time for sensor to advertise
    tokio::time::sleep(BLUETOOTH_SLEEP).await;

    let data = find_sensor_data(adapter).await.ok_or(std::io::Error::new(
        std::io::ErrorKind::NetworkUnreachable,
        "Could not find sensor",
    ))?;
    parse_advertisement_payload(&data)
}

async fn find_sensor_data(adapter: &Adapter) -> Option<Vec<u8>> {
    println!("Finding sensor");
    let peripherals = adapter.peripherals().await.ok()?;
    for peripheral in peripherals {
        if let Ok(Some(props)) = peripheral.properties().await {
            if let Some(data) = props.service_data.get(&SENSOR_UUID) {
                println!("Found 0x181A advertisement");
                return Some(data.clone());
            }
        }
    }
    None
}

fn parse_advertisement_payload(data: &[u8]) -> Result<Measurement, Box<dyn std::error::Error>> {
    if data.len() < 13 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Payload too short: {:?}", data),
        )));
    }

    let mac = &data[0..6];
    let temperature_raw = ((data[6] as u16) << 8) | data[7] as u16;
    let temperature = temperature_raw as f32 / 10.0;
    let humidity = data[8];
    let battery_percent = data[9];
    let battery_mv = u16::from_le_bytes([data[10], data[11]]);
    let packet_counter = data[12];

    println!("--- Decoded Advertisement ---");
    println!(
        "MAC Address: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    println!("Temperature: {:.2} °C", temperature);
    println!("Humidity: {}%", humidity);
    println!("Battery: {}% ({} mV)", battery_percent, battery_mv);
    println!("Packet Counter: {}", packet_counter);
    println!("-----------------------------\n");

    Ok(Measurement {
        location: SENSOR_LOCATION.to_string(),
        temperature: temperature as f64,
        humidity: Some(humidity.into()),
        pressure: None,
    })
}
