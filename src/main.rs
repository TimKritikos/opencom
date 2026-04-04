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

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// Serial Port name
    #[arg(short='s')]
    serial_port: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let cli = Cli::parse();

    const BAUD_RATE: u32 = 9600;

    let mut _port = serialport::new(&cli.serial_port, BAUD_RATE)
        .timeout(Duration::from_secs(2))
        .open()?;

    println!("Opened port: {} @ {} baud", cli.serial_port, BAUD_RATE);

    Ok(())
}
