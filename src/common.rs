use questdb::ingress::{Buffer, Sender, TimestampNanos};

const DATABASE_HOSTNAME: &str = "192.168.0.205";
const DATABASE_PORT: &str = "9000";
const DATABASE_TABLE: &str = "measurements";

#[derive(Debug)]
pub struct Measurement {
    pub location: String,
    pub temperature: f64,
    pub humidity: Option<f64>,
    pub pressure: Option<f64>,
}

pub fn send_measurement(measurement: &Measurement) -> Result<(), questdb::Error> {
    println!("Connecting to database...");
    let mut sender = Sender::from_conf(format!("http::addr={DATABASE_HOSTNAME}:{DATABASE_PORT};"))?;

    println!("Sending measurement {measurement:?} ...");
    let mut buffer = Buffer::new();
    buffer
        .table(DATABASE_TABLE)?
        .symbol("location", &measurement.location)?
        .column_f64("temperature", measurement.temperature)?;

    if let Some(humidity) = measurement.humidity {
        buffer.column_f64("humidity", humidity)?;
    };
    if let Some(pressure) = measurement.pressure {
        buffer.column_f64("pressure", pressure)?;
    };

    buffer.at(TimestampNanos::now())?;
    sender.flush(&mut buffer)?;
    Ok(())
}
