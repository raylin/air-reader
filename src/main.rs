extern crate chrono;
extern crate csv;
extern crate serde;
extern crate serialport;
#[macro_use]
extern crate serde_derive;
extern crate rusqlite;

use chrono::prelude::*;
use serialport::prelude::*;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    local_time: DateTime<Local>,
    pm25: u16,        // ug/m3
    tvoc: u16,        // ppb
    hcho: u16,        // ug/m3
    co2: u16,         // ppm
    temperature: f32, // ℃
    humidity: f32,    // %
}

fn process_bytes(bytes: &[u8; 24]) -> Record {
    let local_time: DateTime<Local> = Local::now();
    let pm25 = (bytes[4] as u16) * 0xff + (bytes[5] as u16);
    let tvoc = (bytes[6] as u16) * 0xff + (bytes[7] as u16);
    let hcho = (bytes[9] as u16) * 0xff + (bytes[10] as u16);
    let co2 = (bytes[12] as u16) * 0xff + (bytes[13] as u16);
    let temperature = ((bytes[14] as f32) * 256.0 + (bytes[15] as f32)) / 10.0;
    let humidity = ((bytes[16] as f32) * 256.0 + (bytes[17] as f32)) / 10.0;

    let record = Record {
        local_time,
        pm25,
        tvoc,
        hcho,
        co2,
        temperature,
        humidity,
    };
    println!("{:?}", record);

    record
}

fn main() {
    let port_name = "/dev/tty.SLAB_USBtoUART";
    let baud_rate = 9600;

    let mut settings: SerialPortSettings = Default::default();
    settings.timeout = Duration::from_millis(10);
    settings.baud_rate = baud_rate;

    let mut port =
        serialport::open_with_settings(&port_name, &settings).expect("Failed to open serial port");

    let head_1 = 0x42;
    let head_2 = 0x4d;

    let mut buf: [u8; 24] = [0; 24];
    let mut serial_buf: Vec<u8> = vec![0; 24];
    let mut cur = 0;

    let mut wtr = csv::Writer::from_path("data.csv").expect("Failed to wrtie to file");

    println!(
        "Ready to receive data on {} at {} rate",
        &port_name, &baud_rate
    );

    loop {
        port.write(&[head_1, head_2, 0xab, 0x00, 0x00, 0x01, 0x3a])
            .expect("Failed to write to serial port");

        match port.read(serial_buf.as_mut_slice()) {
            Ok(bytes) => for i in &serial_buf[..bytes] {
                if cur == 0 && *i != head_1 {
                    continue;
                } else if cur == 1 && *i != head_2 {
                    cur = 0;
                    continue;
                } else if cur == (24 - 1) {
                    wtr.serialize(process_bytes(&buf))
                        .expect("Failed to write Record");
                    wtr.flush().expect("Failed to flush writer");
                    cur = 0;
                } else {
                    buf[cur] = *i;
                    cur = (cur + 1) % 24;
                }
            },
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }

        thread::sleep(Duration::from_secs(10));
    }
}
