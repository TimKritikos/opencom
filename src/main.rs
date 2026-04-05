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
use std::time::Instant;
use std::collections::VecDeque;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use signal_hook::consts::SIGINT;
use signal_hook::flag;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// Serial Port name
    #[arg(short='s')]
    serial_port: String,

    /// Archive JSON file. This will create a json file with all the bytes of the commands sent and responses received with timestamps to parse later
    #[arg(short='a')]
    archive_json_file: Option<PathBuf>,

    /// Report live communication statistics
    #[clap(short='S')]
    live_communication_stats: bool,

    /// Print parsed data as Newline Newline Delimited JSON (NDJSON) to stdout
    #[arg(short='p')]
    print_parsed_data: bool,

    /// Print debug and development information
    #[arg(short='d')]
    print_debug: bool,
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

fn send_command(port:&mut dyn SerialPort, command:&Vec<u8>, size:usize) -> std::io::Result<Vec<u8>>{
    for byte in command {
        port.write_all(&[*byte])?;
        port.flush()?;
    }

    let mut rx_buf = vec![0u8; size];
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

fn initialise_opcom(port:&mut dyn SerialPort, print_debug: bool) -> std::io::Result<()>{
    let init_commands = vec![
        vec![0x02, 0x00, 0x20, 0x07, 0x29],
        vec![0x06, 0x00, 0x02, 0x81, 0x11, 0xf1, 0x81, 0x04, 0x10],
    ];

    for command in init_commands {

        if print_debug {
            eprint!("Sending command [ ",);
            let mut first = 1;
            for byte in &command {
                if first == 1 {
                    first = 0;
                }else{
                    eprint!(", ");
                }
                eprint!("{:02X}",byte);
            }
            eprint!(" ]");
        }

        let received = match send_command(&mut *port, &command, 1024){
            Ok(a) => a,
            Err(e)  => {
                return Err(e);
            }
        };

        if print_debug {
            eprintln!(" OK");

            eprint!("Received {} bytes : ", received.len());

            for byte in received {
                eprint!("{:02X} ", byte);
            }
            eprintln!("\n");
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let cli = Cli::parse();

    if cli.print_debug {
        eprint!("Opening port: {} @ {} baud", cli.serial_port, BAUD_RATE);
    }

    const BAUD_RATE: u32 = 500000;

    let mut port = serialport::new(&cli.serial_port, BAUD_RATE)
        .timeout(Duration::from_millis(150))
        .open()?;

    set_latency_linux(&cli.serial_port,2)?;

    port.write_request_to_send(true)?;

    port.write_data_terminal_ready(true)?;
    thread::sleep(Duration::from_millis(100));

    port.write_request_to_send(true)?;

    port.write_data_terminal_ready(true)?;
    thread::sleep(Duration::from_millis(200));

    port.clear(ClearBuffer::Input)?;

    if cli.print_debug {
        eprintln!(" OK");
    }

    initialise_opcom(&mut *port, cli.print_debug)?;

    let mut request_stat_window = VecDeque::new();
    let mut error_stat_window = VecDeque::new();
    let request_stat_window_size = Duration::from_secs(5);
    let error_stat_window_size = Duration::from_secs(20);
    let mut consecutive_errors = 0;

    let mut archive_json_file:Option<File> = if let Some(archive_json_filename) = cli.archive_json_file {
        Some(File::create(archive_json_filename)?)
    }else{
        None
    };

    if let Some(ref mut file) = archive_json_file {
        writeln!(file,"{{\"data_type\": \"opencom_archive_log_file\",\"data_structure_version\":\"{}\",\"data\":[",env!("CARGO_PKG_VERSION"))?;
    }

    let mut list_comma=false;
    let terminate_flag = Arc::new(AtomicBool::new(false));
    flag::register(SIGINT, Arc::clone(&terminate_flag))?;

    loop{
        let get_engine_data_command = vec![0x07, 0x00, 0x01, 0x82, 0x11, 0xf1, 0x21, 0x01, 0xa6, 0x54];

        match send_command(&mut *port, &get_engine_data_command, 64){
            Ok(received) => {
                //TODO: Don't calculate those if flags not enabled
                let stat_timestamp = Instant::now();
                let clock_timestamp = SystemTime::now();

                if let Some(ref mut file) = archive_json_file {
                    let data_timestamp = clock_timestamp.duration_since(UNIX_EPOCH)?;
                    write!(file,"{}{{\"timestamp\":{}.{},\"command\":[",if list_comma{","}else{""},data_timestamp.as_secs(),data_timestamp.subsec_nanos())?;
                    let mut comma=false;
                    for byte in &get_engine_data_command {
                        write!(file, "{}{}",if comma{","}else{""}, byte)?;
                        if !comma {
                            comma=true;
                        }
                    }
                    write!(file,"],\"response\":[",)?;
                    comma=false;
                    for byte in &received {
                        write!(file, "{}{}",if comma{","}else{""}, byte)?;
                        if !comma {
                            comma=true;
                        }
                    }
                    if !list_comma {
                        list_comma=true;
                    }
                    writeln!(file,"]}}")?;
                }

                // Check if we get valid data and act if we don't
                let valid_data = if  received.len() == 64 {
                    let zero =                     received[ 1];
                    let checksum1 =                received[62];
                    let _unkown_checksum2 =        received[63];

                    let mut b1:u8 = 0;
                    for byte in &received[9..62]{
                        b1=b1.wrapping_add(*byte);
                    }

                    if  checksum1 != b1 || zero != 0 {
                        eprintln!("Invalid checksum!\n");
                        thread::sleep(Duration::from_millis(400));
                        false
                    }else{
                        consecutive_errors=0;
                        true
                    }
                }else{
                    eprintln!("Invalid response size!");
                    if cli.live_communication_stats {
                        error_stat_window.push_back(stat_timestamp);
                    }
                    consecutive_errors += 1;
                    false
                };

                if consecutive_errors > 5 {
                    initialise_opcom(&mut *port, cli.print_debug)?;
                    consecutive_errors = 0;
                }

                // Parse and print values
                if valid_data && cli.print_parsed_data {
                    let battery_voltage =          received[22];
                    let throttle_position_sensor = received[36];

                    eprintln!("Throttle position sensor: {}%",((throttle_position_sensor as u16 *100)/255));
                    eprintln!("Battery voltage: {}.{}V", battery_voltage / 10, battery_voltage % 10);
                }

                if cli.live_communication_stats {
                    request_stat_window.push_back(stat_timestamp);

                    // Calculate Stats
                    while let Some(&front) = request_stat_window.front() {
                        if stat_timestamp.duration_since(front) > request_stat_window_size {
                            request_stat_window.pop_front();
                        } else {
                            break;
                        }
                    }
                    while let Some(&front) = error_stat_window.front() {
                        if stat_timestamp.duration_since(front) > error_stat_window_size {
                            error_stat_window.pop_front();
                        } else {
                            break;
                        }
                    }

                    eprintln!("Sample rate: {:.1}Hz", request_stat_window.len() as f64 / request_stat_window_size.as_secs_f64());
                    eprintln!("Error rate: {:.2}Hz", error_stat_window.len() as f64 / error_stat_window_size.as_secs_f64());
                }

                if cli.print_debug {
                    eprint!("Raw received bytes: ({})",received.len());
                    for byte in received {
                        eprint!("{:02X} ", byte);
                    }
                    eprintln!();
                }
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => {
                eprintln!("Error: {:?}",e);
            }
        }

        if terminate_flag.load(Ordering::Relaxed) {
            if let Some(ref mut file) = archive_json_file {
                writeln!(file,"]}}")?;
            }
            break;
        }
    }
    Ok(())
}
