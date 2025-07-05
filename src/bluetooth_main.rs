mod common;

use crate::common::Measurement;
use crate::common::send_measurement;
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Adapter;
use btleplug::platform::Manager;
use std::time::Duration;

const BLUETOOTH_SLEEP: Duration = Duration::from_secs(10);
const SENSOR_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x0000181a00001000800000805f9b34fb);

const SENSORS: &[(&str, &str)] = &[
    ("A4:C1:38:12:34:56", "kitchen"),
    ("A4:C1:38:45:67:89", "bathroom"),
];

#[tokio::main]
async fn main() {
    let bluetooth_measurements = listen_bluetooth_measurements()
        .await
        .expect("Reading bluetooth measurements should succeed");
    println!("Measurements: {:#?}", bluetooth_measurements);
    let result: Result<Vec<()>, _> = bluetooth_measurements
        .into_iter()
        .map(|measurement| send_measurement(&measurement))
        .collect();
    result.expect("Sending bluetooth measurements should succeed");
    println!("All done.");
}

async fn listen_bluetooth_measurements() -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.first().expect("Should find bluetooth adapeter");

    println!("Starting BLE scan ...");
    adapter.start_scan(ScanFilter::default()).await?;

    // Give time for sensor to advertise
    tokio::time::sleep(BLUETOOTH_SLEEP).await;

    get_all_sensors_measurements(adapter).await
}

async fn get_all_sensors_measurements(
    adapter: &Adapter,
) -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    println!("Finding sensors");
    let peripherals = adapter.peripherals().await.ok().ok_or(std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "Could not get peripherals",
    ))?;
    let mut measurements = Vec::new();
    for peripheral in peripherals {
        if let Ok(Some(props)) = peripheral.properties().await {
            if let Some(data) = props.service_data.get(&SENSOR_UUID) {
                println!("Found 0x181A advertisement");
                let location = get_location(data);
                if let Ok(location) = location {
                    if let Ok(measurement) = parse_advertisement_payload(&location, data) {
                        measurements.push(measurement);
                    } else {
                        println!("Could not parse payload for location {}", location);
                    }
                } else {
                    println!("Could not get location for {:?}", data);
                }
            }
        }
    }
    Ok(measurements)
}

fn get_location(data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    if data.len() < 13 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Payload too short: {:?}", data),
        )));
    }
    let mac = &data[0..6];
    let mac = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    SENSORS
        .into_iter()
        .find(|sensor| sensor.0 == mac)
        .map(|sensor| sensor.1.to_string())
        .ok_or("No matching location".into())
}

fn parse_advertisement_payload(
    location: &str,
    data: &[u8],
) -> Result<Measurement, Box<dyn std::error::Error>> {
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
        location: location.to_string(),
        temperature: temperature as f64,
        humidity: Some(humidity.into()),
        pressure: None,
    })
}
