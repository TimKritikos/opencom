use serialport::SerialPort;
use std::any::Any;
use crate::EcuSubsystem;
use crate::Command;
//use crate::send_command;

use crate::opcom_communication;

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
        match opcom_communication::send_command(&mut *port, Self::init_command(self), print_debug){
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn query(&self, port:&mut dyn SerialPort, print_debug:bool) -> std::io::Result<Vec<u8>> {
        opcom_communication::send_command(&mut *port, Self::request_command(self), print_debug)
    }

    fn decode(&self, _data: &[u8]) -> std::io::Result<Box<dyn Any>> {
        Ok(Box::new(ChassisData {
        }))
    }
}

