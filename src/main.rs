mod common;

use crate::common::Measurement;
use crate::common::send_measurement;
use anyhow::anyhow;
use open_meteo_rs::Location;
use std::process::Command;

const OPEN_METEO_LOCATION: Location = Location {
    lat: 59.32938,
    lng: 18.06871,
};
const SENSORS_BINARY: &str = "bme280";
const SENSORS_LOCATION: &str = "dummy";

#[tokio::main]
async fn main() {
    let measurement = read_sensors().expect("Reading measurement should succeed");
    send_measurement(&measurement).expect("Sending measurement should succeed");
    let open_meteo_measurement = get_open_meteo_data()
        .await
        .expect("Getting open meteo data should succeed");
    send_measurement(&open_meteo_measurement).expect("Sending open meteo data should succeed");
    println!("All done.");
}

fn read_sensors() -> Result<Measurement, std::io::Error> {
    println!("Reading sensors...");
    let output = Command::new(SENSORS_BINARY).output()?.stdout;
    let line = String::from_utf8_lossy(&output);
    parse_reading_line(line.trim()).ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Couldn't parse data",
    ))
}

fn parse_reading_line(line: &str) -> Option<Measurement> {
    let mut split = line.split_whitespace();
    let temperature = split.next()?;
    let pressure = split.next()?;
    let humidity = split.next()?;

    let temperature = temperature.parse().ok()?;
    let pressure = Some(pressure.parse().ok()?);
    let humidity = Some(humidity.parse().ok()?);

    Some(Measurement {
        location: String::from(SENSORS_LOCATION),
        temperature,
        humidity,
        pressure,
    })
}

async fn get_open_meteo_data() -> Result<Measurement, Box<dyn std::error::Error>> {
    let client = open_meteo_rs::Client::new();
    let mut opts = open_meteo_rs::forecast::Options {
        location: OPEN_METEO_LOCATION,
        elevation: Some(open_meteo_rs::forecast::Elevation::Nan),
        temperature_unit: Some(open_meteo_rs::forecast::TemperatureUnit::Celsius),
        wind_speed_unit: Some(open_meteo_rs::forecast::WindSpeedUnit::Ms),
        precipitation_unit: Some(open_meteo_rs::forecast::PrecipitationUnit::Millimeters),
        cell_selection: Some(open_meteo_rs::forecast::CellSelection::Land),
        ..Default::default()
    };

    opts.current.push("temperature_2m".into());
    let res = client.forecast(opts).await?;
    let current = res.current.ok_or(anyhow!("No current data"))?;
    let temperature = current
        .values
        .get("temperature_2m")
        .ok_or(anyhow!("No temperature data"))?
        .value
        .as_f64()
        .ok_or(anyhow!("Not a number!"))?;
    Ok(Measurement {
        location: String::from("outside"),
        temperature,
        humidity: None,
        pressure: None,
    })
}
