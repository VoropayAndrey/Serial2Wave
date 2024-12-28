use std::io::{self, Read};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use serialport::SerialPort;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::Local;
mod constants;
mod parser;
mod utils;


fn current_millis() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    millis.to_string()
}

fn clear_serial_buffer(port: &mut Box<dyn SerialPort>, bytes_to_clear: usize) {
    let mut discard_buffer = vec![0u8; bytes_to_clear];
    let mut total_bytes_read = 0;

    while total_bytes_read < bytes_to_clear {
        match port.read(&mut discard_buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    break;
                }
                total_bytes_read += bytes_read;
            }
            Err(e) => {
                eprintln!("Error while clearing buffer: {}", e);
                break;
            }
        }
    }
}

fn main() -> io::Result<()> {
    let port = serialport::new(constants::common::SERIAL_PORT, 
            constants::common::BAUDRATE)
        .timeout(Duration::from_secs(1))
        .open();

    let sync_vec: Vec<u8> = vec![0xFF, 0x01, 0xFF, 0x02, 0xFF, 0x03, 0xFF, 0x04];

    let parser = Arc::new(Mutex::new(parser::parser::Parser::new(sync_vec)));

    // Set a callback to handle parsed frames
    {
        let mut parser_lock = parser.lock().unwrap();
        parser_lock.set_callback(|frame_type, data| {
            match frame_type {
                parser::parser::FrameType::LogData => {
                    let now = Local::now();
                    let filtered_string: String = data.iter()
                        .filter(|&&b| b.is_ascii()) // Keep only ASCII bytes
                        .map(|&b| b as char)        // Convert each byte to a char
                        .collect(); 
                    println!("{} - {}", now.format("%Y-%m-%d %H:%M:%S%.3f"), filtered_string)
                },
                parser::parser::FrameType::AudioData => {
                    let now = Local::now();
                    let frame_number: u32 = (data[4000] as u32)
                    | ((data[4001] as u32) << 8)
                    | ((data[4002] as u32) << 16)
                    | ((data[4003] as u32) << 24);
                    println!("{} - AUDIO Frame Received Length: {}, frame_number: {}", now.format("%Y-%m-%d %H:%M:%S%.3f"), data.len(), frame_number); 
                },
            }
        });
    }

    // Start processing thread
    parser::parser::Parser::start(Arc::clone(&parser));

    match port {
        Ok(mut port) => {
            println!("Listening on {} at {} baud...", constants::common::SERIAL_PORT, 
                constants::common::BAUDRATE);
            
            // Clear the serial buffer before starting
            clear_serial_buffer(&mut port, constants::common::SERIAL_READ_SIZE);

            let mut read_buffer: [u8; constants::common::SERIAL_READ_SIZE] = [0u8; constants::common::SERIAL_READ_SIZE]; // choose an appropriate size
            loop {
                match port.read(&mut read_buffer) {
                    Ok(n) if n > 0 => {
                        let mut parser = parser.lock().expect("Failed to lock parser mutex");
                        parser.push_data(&read_buffer[..n]);
                    }
                    Ok(_) => {
                        // n == 0 means EOF or no data; depending on serial config
                    }
                    Err(e) => {
                        eprintln!("Serial read error: {}", e);
                        // Possibly break or handle error
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open serial port: {}", e);
        }
    }

    Ok(())
}
