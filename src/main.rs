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

use clap::{Parser,ValueEnum,Args};
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
use std::any::Any;
use serde::Deserialize;

//#[derive(Clone,ValueEnum,Debug)]
#[derive(ValueEnum, Clone, PartialEq)]
enum ScanModule {
    Engine,
    Chassis,
}
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Acquisition of data from the ECU
    Acquire(AcquireArgs),

    /// Decode previously recorded raw data
    Decode(ReplayDecodeArgs),

    /// Replay recorded data at real-time speed
    Replay(ReplayDecodeArgs),
}

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// Report live communication statistics
    #[clap(short='S', long)]
    live_communication_stats: bool,

    /// Print parsed data as Newline Newline Delimited JSON (NDJSON) to stdout
    #[arg(short='p', long)]
    print_parsed_data: bool,

    /// Print debug and development information
    #[arg(short='d', long)]
    print_debug: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args)]
pub struct AcquireArgs {
    /// Serial Port name
    #[arg(short='s', long)]
    serial_port: String,

    /// Archive JSON file. This will create a json file with all the bytes of the commands sent and responses received with timestamps to parse later
    #[arg(short='a', long)]
    archive_json_file: Option<PathBuf>,

    /// Comma sepparated list of modules to scan
    #[arg(short='m',value_enum,value_delimiter = ',', default_values_t = [ScanModule::Engine], long)]
    modules: Vec<ScanModule>,
}

#[derive(Args)]
pub struct ReplayDecodeArgs {
    /// Input archive JSON file
    #[arg(short='i', long)]
    input_archive: PathBuf,
}

#[derive(Debug, Clone, Copy)]
#[derive(PartialEq)]
pub struct Command {
    pub request: &'static [u8],
    pub response_len: usize,
}

pub trait EcuSubsystem {
    fn init_command(&self) -> Command;
    fn request_command(&self) -> Command;
    fn init(&self, port:&mut dyn SerialPort, print_debug:bool) -> std::io::Result<()>;
    fn query(&self, port:&mut dyn SerialPort, print_debug:bool) -> std::io::Result<Vec<u8>>;
    fn decode(&self, data: &[u8]) -> std::io::Result<Box<dyn Any>>;
}

pub struct Engine;

pub struct EngineData {
    pub throttle_position: f32,
    pub throttle_position_voltage: f32,
    pub battery_voltage: f32,
    pub air_fule_ratio: f32,
    pub idle_air_control_valve: f32,
    pub injection_pulse_timing: f32,
    pub o2_block_learn_multiplier_cell_number: u8,
    pub rotations_per_minute: u16,
}

impl EcuSubsystem for Engine {

    fn init_command(&self) -> Command {
        Command {
            request: &[0x06, 0x00, 0x02, 0x81, 0x11, 0xf1, 0x81, 0x04, 0x10],
            response_len: 17,
        }
    }
    fn request_command(&self) -> Command {
        Command {
            request: &[0x07, 0x00, 0x01, 0x82, 0x11, 0xf1, 0x21, 0x01, 0xa6, 0x54],
            response_len: 64,
        }
    }

    fn init(&self, port:&mut dyn SerialPort, print_debug:bool) -> std::io::Result<()> {
        match send_command(&mut *port, Self::init_command(self), print_debug){
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn query(&self, port:&mut dyn SerialPort, print_debug:bool) -> std::io::Result<Vec<u8>> {
        send_command(&mut *port, Self::request_command(self), print_debug)
    }

    fn decode(&self, data: &[u8]) -> std::io::Result<Box<dyn Any>> {

        let battery_voltage =                       data[22];
        let throttle_position_voltage =             data[35];
        let throttle_position_sensor =              data[36];
        let injection_pulse_timing =                data[39];
        let idle_air_control_valve =                data[40];
        let o2_block_learn_multiplier_cell_number = data[45];
        let air_fule_ratio =                        data[49];
        let rotations_per_minute =                  data[38];

        Ok(Box::new(EngineData {
            throttle_position: ((throttle_position_sensor as f32 *100.0)/255.0),
            battery_voltage: battery_voltage as f32 / 10.0,
            air_fule_ratio: air_fule_ratio as f32 / 10.0,
            idle_air_control_valve: ((idle_air_control_valve as f32 *100.0)/255.0),
            throttle_position_voltage: throttle_position_voltage as f32 * 0.0195, // TODO: The multiplier is an estimate
            injection_pulse_timing: injection_pulse_timing as f32 * 0.086, // TODO The multiplier is an estimate
            o2_block_learn_multiplier_cell_number: o2_block_learn_multiplier_cell_number,
            rotations_per_minute: rotations_per_minute as u16 * 25,
        }))
    }
}

pub struct Chassis;

pub struct ChassisData {
}

impl EcuSubsystem for Chassis {

    fn init_command(&self) -> Command {
        Command {
            request: &[0x06, 0x00, 0x02, 0x81, 0x28, 0xf1 ,0x81 ,0x1b ,0x3e],
            response_len: 17,
        }
    }
    fn request_command(&self) -> Command {
        Command {
            request: &[0x07, 0x00, 0x01, 0x82, 0x28, 0xf1, 0x21, 0x01, 0xbd, 0x82],
            response_len: 38,
        }
    }

    fn init(&self, port:&mut dyn SerialPort, print_debug:bool) -> std::io::Result<()> {
        match send_command(&mut *port, Self::init_command(self), print_debug){
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn query(&self, port:&mut dyn SerialPort, print_debug:bool) -> std::io::Result<Vec<u8>> {
        send_command(&mut *port, Self::request_command(self), print_debug)
    }

    fn decode(&self, _data: &[u8]) -> std::io::Result<Box<dyn Any>> {
        Ok(Box::new(ChassisData {
        }))
    }
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

fn send_command(port:&mut dyn SerialPort, command:Command, print_debug: bool) -> std::io::Result<Vec<u8>>{
    if print_debug {
        eprint!("Sending command [ ",);
        let mut first = 1;
        for byte in command.request {
            if first == 1 {
                first = 0;
            }else{
                eprint!(", ");
            }
            eprint!("{:02X}",byte);
        }
        eprint!(" ]");
    }

    for byte in command.request {
        port.write_all(&[*byte])?;
        port.flush()?;
    }

    let mut rx_buf = vec![0u8; command.response_len];
    let mut total_read = 0;

    loop {
        match port.read(&mut rx_buf[total_read..]) {
            Ok(n) => {
                total_read += n;
                if total_read >= rx_buf.len() {
                    if print_debug {
                        eprintln!(" OK");

                        eprint!("Received {} bytes : ", total_read);

                        for byte in rx_buf.iter().take(total_read) {
                            eprint!("{:02X} ", byte);
                        }
                        eprintln!("\n");
                    }
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                if print_debug {
                    eprintln!(" Ignored due to SIGINT");
                }
                break;
            }
            Err(e) => {
                if print_debug {
                    eprintln!(" ERR");
                }
                return Err(e);
            }
        }
    }

    let received = &rx_buf[..total_read];

    Ok(Vec::from(received))
}

#[derive(Deserialize)]
struct ArchiveJsonDataPoint{
    timestamp: f32,
    command: Vec<u8>,
    response: Vec<u8>,
}

#[derive(Deserialize)]
struct ArchiveJson{
    data_type: String,
    data_structure_version: String,
    data: Vec<ArchiveJsonDataPoint>,
}

fn main_loop(input_archive:Option<PathBuf>, replay_realtime:bool, output_archive:Option<PathBuf>, mut serial_port:Option<&mut dyn SerialPort>, modules: Vec<ScanModule>, print_debug:bool, print_parsed_data:bool, live_communication_stats:bool) -> std::io::Result<()>{
    let mut request_stat_window = VecDeque::new();
    let mut error_stat_window = VecDeque::new();
    let request_stat_window_size = Duration::from_secs(5);
    let error_stat_window_size = Duration::from_secs(20);
    let mut consecutive_errors = 0;

    let mut output_archive_json_file:Option<File> = if let Some(output_archive) = output_archive {
        Some(File::create(output_archive)?)
    }else{
        None
    };

    if let Some(ref mut file) = output_archive_json_file {
        writeln!(file,"{{\"data_type\": \"opencom_archive_log_file\",\"data_structure_version\":\"{}\",\"data\":[",env!("CARGO_PKG_VERSION"))?;
    }

    let terminate_flag = Arc::new(AtomicBool::new(false));
    flag::register(SIGINT, Arc::clone(&terminate_flag))?;

    let mut list_comma=false;

    let mut old_subsystem: &ScanModule = &ScanModule::Engine;
    let mut first_init = false;

    let mut archive_json_iterator = if let Some(input_archive_file) = input_archive {
        let data = std::fs::read_to_string(input_archive_file).unwrap();
        let cfg: ArchiveJson = serde_json::from_str(&data)?;
        if cfg.data_type != "opencom_archive_log_file".to_string() {
            eprintln!("Input archive isn't the correct type");
            return Ok(());//TODO: Fix error handling
        }
        if cfg.data_structure_version != "0.1.0".to_string() { // TODO: Fix this check
            eprintln!("Input archive file isn't written by a compatible software version");
            return Ok(());//TODO: Fix error handling
        }
        let iterator = IntoIterator::into_iter(cfg.data);
        Some(iterator)
    }else{
        None
    };

    'main_loop: loop{
        for subsystem in &modules {

            let subsystem_code: Box<dyn EcuSubsystem> = match subsystem{
                ScanModule::Engine => Box::new(Engine),
                ScanModule::Chassis => Box::new(Chassis),
            };

            if let Some(ref mut port) = serial_port {
                if subsystem != old_subsystem || !first_init {
                    subsystem_code.init(*port, print_debug)?;
                }
                if !first_init{
                    first_init = true;
                }
            }

            old_subsystem = subsystem;
            let response = if let Some(ref mut port) = serial_port {
                match subsystem_code.query( *port, print_debug){
                    Ok(received) => {
                        Some(received)
                    },
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                        None
                    }
                    Err(e) => {
                        eprintln!("Error: {:?}",e);
                        break 'main_loop;
                    }
                }
            }else if let Some(ref mut iterator) = archive_json_iterator {
                match iterator.next() {
                    Some(i) => {
                        if i.command !=  [0x07, 0x00, 0x01, 0x82, 0x11, 0xf1, 0x21, 0x01, 0xa6, 0x54]{
                            eprintln!("Error: archive contains data from subsystem other than the engine one and it's not currently supported");
                            break 'main_loop // TODO: do proper error handling
                        }
                        Some(i.response)
                    }
                    None => {
                        break 'main_loop
                    }
                }
            }else{
                None
            };

            if let Some(received) = response {
                //TODO: Don't calculate those if flags not enabled
                let stat_timestamp = Instant::now();
                let clock_timestamp = SystemTime::now();

                if let Some(ref mut file) = output_archive_json_file {
                    let data_timestamp = clock_timestamp.duration_since(UNIX_EPOCH).unwrap();
                    write!(file,"{}{{\"timestamp\":{}.{},\"command\":[",if list_comma{","}else{""},data_timestamp.as_secs(),data_timestamp.subsec_nanos())?;
                    let mut comma=false;
                    for byte in subsystem_code.request_command().request {
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
                let valid_data = if received.len() > 9 {
                    let zero =                     received[ 1];
                    let checksum1 =                received[received.len()-2];
                    let _unkown_checksum2 =        received[received.len()-1];

                    let mut b1:u8 = 0;
                    for byte in &received[9..received.len()-2]{
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
                    if live_communication_stats {
                        error_stat_window.push_back(stat_timestamp);
                    }
                    consecutive_errors += 1;
                    false
                };

                if let Some(ref mut port) = serial_port && consecutive_errors > 5 {
                    subsystem_code.init(*port,print_debug)?;
                    consecutive_errors = 0;
                }

                // Parse and print values
                if valid_data && print_parsed_data {
                    let parsed = subsystem_code.decode(&received)?;
                    if let Ok(parsed) = parsed.downcast::<EngineData>() {
                        eprintln!("Throttle position sensor: {}%",parsed.throttle_position );
                        eprintln!("Throttle position sensor Voltage: {}V",parsed.throttle_position_voltage );
                        eprintln!("Battery voltage: {}V", parsed.battery_voltage );
                        eprintln!("Air/Fuel Ratio: {}", parsed.air_fule_ratio );
                        eprintln!("Idle air control valve: {}%", parsed.idle_air_control_valve );
                        eprintln!("Injection pulse: {}ms", parsed.injection_pulse_timing );
                        eprintln!("O2 Block Learn Multiplier cell number: {}", parsed.o2_block_learn_multiplier_cell_number );
                        eprintln!("RPM : {}", parsed.rotations_per_minute );
                    }
                }

                if live_communication_stats {
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

                if print_debug {
                    eprint!("Raw received bytes: ({})",received.len());
                    for byte in received {
                        eprint!("{:02X} ", byte);
                    }
                    eprintln!();
                }
            }


            if terminate_flag.load(Ordering::Relaxed) {
                if let Some(ref mut file) = output_archive_json_file {
                    writeln!(file,"]}}")?;
                }
                break 'main_loop;
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let cli = Cli::parse();

    match cli.command {
        Commands::Acquire(args) => {
            // Initialise communication with the OpCom/VauxCom adapter

            if cli.print_debug {
                eprint!("Opening port: {} @ {} baud", args.serial_port, BAUD_RATE);
            }

            const BAUD_RATE: u32 = 500000;

            let mut port = serialport::new(&args.serial_port, BAUD_RATE)
                .timeout(Duration::from_millis(1000))
                .open()?;

            set_latency_linux(&args.serial_port,2)?;

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

            let _ = send_command(&mut *port, Command { request: &[0x02, 0x00, 0x20, 0x07, 0x29], response_len: 200, },cli.print_debug);

            main_loop(None, false, args.archive_json_file, Some(&mut *port), args.modules, cli.print_debug, cli.print_parsed_data, cli.live_communication_stats)?;
        },
        Commands::Decode(args) => {
            main_loop(Some(args.input_archive), false, None, None, Vec::from([ScanModule::Engine]), cli.print_debug, cli.print_parsed_data, cli.live_communication_stats)?;
        }
        Commands::Replay(args) => {
            //main_loop(Some(args.input_archive), true, None, None, Vec::new(), cli.print_debug, cli.print_parsed_data, cli.live_communication_stats)?;
            eprintln!("Not currently supoprted");
        }
    }

    Ok(())
}
