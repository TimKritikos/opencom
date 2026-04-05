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
use std::thread;
use serialport::ClearBuffer;
use serialport::SerialPort;

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

fn send_command(port:&mut dyn SerialPort, command:Vec<u8> ) -> std::io::Result<Vec<u8>>{
    for byte in command {
        port.write_all(&[byte])?;
        port.flush()?;
    }

    let mut rx_buf = vec![0u8; 1024];
    let mut total_read = 0;

    loop {
        match port.read(&mut rx_buf[total_read..]) {
            Ok(n) => {
                total_read += n;
                if total_read >= rx_buf.len() {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                break;
            }
            Err(e) => return Err(e),
        }
    }

    let received = &rx_buf[..total_read];

    Ok(Vec::from(received))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let cli = Cli::parse();

    print!("Opening port: {} @ {} baud", cli.serial_port, BAUD_RATE);

    const BAUD_RATE: u32 = 500000;

    let mut port = serialport::new(&cli.serial_port, BAUD_RATE)
        .timeout(Duration::from_millis(400))
        .open()?;

    set_latency_linux(&cli.serial_port,2)?;

    port.write_request_to_send(true)?;

    port.write_data_terminal_ready(true)?;
    thread::sleep(Duration::from_millis(100));

    port.write_request_to_send(true)?;

    port.write_data_terminal_ready(true)?;
    thread::sleep(Duration::from_millis(200));

    port.clear(ClearBuffer::Input)?;

    println!(" OK");

    let init_commands = vec![
        vec![0x01, 0x00, 0xab, 0xac],
        vec![0x01, 0x00, 0xaa, 0xab],
        vec![0x02, 0x00, 0xac, 0x01, 0xaf],
        vec![0x01, 0x00, 0x74, 0x75],
        vec![0x04, 0x00, 0x73, 0x01, 0x00, 0xfb, 0x73],
        vec![0x04, 0x00, 0x73, 0x02, 0x30, 0xec, 0x95],
        vec![0x02, 0x00, 0x73, 0x04, 0x79],
        vec![0x02, 0x00, 0x82, 0x02, 0x86],
        vec![0x04, 0x00, 0x73, 0x02, 0x50, 0xe7, 0xb0],
        vec![0x04, 0x00, 0x73, 0x02, 0x00, 0xfb, 0x74],
        vec![0x02, 0x00, 0x20, 0x07, 0x29],
        vec![0x06, 0x00, 0x02, 0x81, 0x11, 0xf1, 0x81, 0x04, 0x10],
        vec![0x07, 0x00, 0x01, 0x82, 0x11, 0xf1, 0x1a, 0x81, 0x1f, 0x46],
        vec![0x07, 0x00, 0x01, 0x82, 0x11, 0xf1, 0x1a, 0x80, 0x1e, 0x44],
    ];

    for command in init_commands {

        print!("Sending command [ ",);
        let mut first = 1;
        for byte in &command {
            if first == 1 {
                first = 0;
            }else{
                print!(", ");
            }
            print!("{:02X}",byte);
        }
        print!(" ]");

        let received = send_command(&mut *port, command).unwrap();

        println!(" OK");

        print!("Received {} bytes : ", received.len());

        for byte in received {
            print!("{:02X} ", byte);
        }
        println!("\n");
    }

    loop{
        let get_engine_data_command = vec![0x07, 0x00, 0x01, 0x82, 0x11, 0xf1, 0x21, 0x01, 0xa6, 0x54];

        let received = send_command(&mut *port, get_engine_data_command).unwrap();

        if  received.len() == 64 {
            let battery_voltage =          received[22];
            let throttle_position_sensor = received[36];
            let checksum1 =                received[62];
            let _unkown_checksum2 =        received[63];

            // Check checksum
            let mut b1:u8 = 0;
            for byte in &received[9..62]{
                b1=b1.wrapping_add(*byte);
            }

            if  checksum1 != b1 {
                println!("Invalid checksum!\n");
            }else{
                println!("Throttle position sensor: {}%",((throttle_position_sensor as u16 *100)/255));
                println!("Battery voltage: {}.{}V", battery_voltage / 10, battery_voltage % 10);
            }
        }else{
            print!("Invalid response!");
        }

        //print!("Raw bytes: ({})",received.len());
        for byte in received {
            print!("{:02X} ", byte);
        }
        println!("\n");


    }

}
