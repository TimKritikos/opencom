use serialport::SerialPort;
use std::any::Any;
use crate::EcuSubsystem;
use crate::Command;
use serde::Serialize;

//mod opcom_communication;
use crate::opcom_communication;

pub struct Engine;

#[derive(Serialize)]
pub struct EngineData {
    pub throttle_position_percentage: f32,
    pub throttle_position_voltage: f32,
    pub battery_voltage: f32,
    pub air_fuel_ratio: f32,
    pub idle_air_control_valve_percentage: f32,
    pub injection_pulse_timing_milliseconds: f32,
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
        match opcom_communication::send_command(&mut *port, Self::init_command(self), print_debug){
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn query(&self, port:&mut dyn SerialPort, print_debug:bool) -> std::io::Result<Vec<u8>> {
        opcom_communication::send_command(&mut *port, Self::request_command(self), print_debug)
    }

    fn decode(&self, data: &[u8]) -> std::io::Result<Box<dyn Any>> {
        if data.len() == 64 {
            let battery_voltage =                       data[22];
            let throttle_position_voltage =             data[35];
            let throttle_position_percentage =          data[36];
            let injection_pulse_timing_milliseconds =   data[39];
            let idle_air_control_valve_percentage =     data[40];
            let o2_block_learn_multiplier_cell_number = data[45];
            let air_fuel_ratio =                        data[49];
            let rotations_per_minute =                  data[38];

            Ok(Box::new(EngineData {
                throttle_position_percentage: ((throttle_position_percentage as f32 *100.0)/255.0),
                battery_voltage: battery_voltage as f32 / 10.0,
                air_fuel_ratio: air_fuel_ratio as f32 / 10.0,
                idle_air_control_valve_percentage: ((idle_air_control_valve_percentage as f32 *100.0)/255.0),
                throttle_position_voltage: throttle_position_voltage as f32 * 0.0195, // TODO: The multiplier is an estimate
                injection_pulse_timing_milliseconds: injection_pulse_timing_milliseconds as f32 * 0.086, // TODO The multiplier is an estimate
                o2_block_learn_multiplier_cell_number: o2_block_learn_multiplier_cell_number,
                rotations_per_minute: rotations_per_minute as u16 * 25,
            }))
        }else{
            Err(std::io::Error::other("invalid command size"))
        }
    }
}
