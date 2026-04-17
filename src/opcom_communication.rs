use serialport::SerialPort;

use crate::Command;

pub fn send_command(port:&mut dyn SerialPort, command:Command, print_debug: bool) -> std::io::Result<Vec<u8>>{
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
