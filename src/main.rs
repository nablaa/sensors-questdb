use std::process::Command;

use questdb::ingress::{Buffer, Sender, TimestampNanos};

const SENSORS_BINARY: &str = "bme280";
const LOCATION: &str = "closet";
const DATABASE_HOSTNAME: &str = "192.168.0.205";
const DATABASE_PORT: &str = "9000";
const DATABASE_TABLE: &str = "measurements";

fn main() {
    let measurement = read_sensors().expect("Reading measurement should succeed");
    send_measurement(&measurement).expect("Sending measurement should succeed");
    log::info!("All done.");
}

struct Measurement {
    temperature: f64,
    humidity: f64,
    pressure: f64,
}

fn read_sensors() -> Result<Measurement, std::io::Error> {
    log::info!("Reading sensors...");
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
    let pressure = pressure.parse().ok()?;
    let humidity = humidity.parse().ok()?;

    Some(Measurement {
        temperature,
        humidity,
        pressure,
    })
}

fn send_measurement(measurement: &Measurement) -> Result<(), questdb::Error> {
    log::info!("Connecting to database...");
    let mut sender = Sender::from_conf(format!("http::addr={DATABASE_HOSTNAME}:{DATABASE_PORT};"))?;

    log::info!("Sending measurement...");
    let mut buffer = Buffer::new();
    buffer
        .table(DATABASE_TABLE)?
        .symbol("location", LOCATION)?
        .column_f64("temperature", measurement.temperature)?
        .column_f64("humidity", measurement.humidity)?
        .column_f64("pressure", measurement.pressure)?
        .at(TimestampNanos::now())?;
    sender.flush(&mut buffer)?;
    Ok(())
}
