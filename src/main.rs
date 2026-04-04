/* main.rs

   This file is part of the opencom project

   Copyright (c) 2026 Efthymios Kritikos

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU General Public License for more details.

   You should have received a copy of the GNU General Public License
   along with this program.  If not, see <http://www.gnu.org/licenses/>.  */

use clap::{Parser};
use std::time::Duration;
use std::path::{PathBuf};
use std::fs::File;
use std::io::Write;

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// Serial Port name
    #[arg(short='s')]
    serial_port: String,
}

fn set_latency_linux(device: &String, latency: u8) -> std::io::Result<()> {
    let location = PathBuf::from(device);
    let path = format!(
        "/sys/bus/usb-serial/devices/{}/latency_timer",
        location.file_name().unwrap().to_os_string().into_string().unwrap()

    );

    let mut file = File::create(path)?;
    write!(file, "{}", latency)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let cli = Cli::parse();

    const BAUD_RATE: u32 = 500000;

    let mut port = serialport::new(&cli.serial_port, BAUD_RATE)
        .timeout(Duration::from_secs(2))
        .open()?;

    set_latency_linux(&cli.serial_port,2)?;

    port.write_request_to_send(true)?;

    port.write_data_terminal_ready(true)?;

    println!("Opened port: {} @ {} baud", cli.serial_port, BAUD_RATE);

    Ok(())
}
