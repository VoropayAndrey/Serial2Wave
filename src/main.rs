use std::io::{self, Read};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serialport::SerialPort;
use chrono::Local;
use argh::FromArgs;
use once_cell::sync::Lazy;
use hound;
use ctrlc;
use byteorder_slice::{BigEndian, ByteOrder, LittleEndian};

mod constants;
mod parser;
mod utils;

/// Default configuration file path.
const DEFAULT_CONFIG: &str = "config.json";

const spec: hound::WavSpec = hound::WavSpec {
    channels: 1,
    sample_rate: 48000,
    bits_per_sample: 16,
    sample_format: hound::SampleFormat::Int,
};

/// An example CLI tool.
#[derive(FromArgs)]
struct Args {
    /// path to the configuration file
    #[argh(option, short = 'c', default = "DEFAULT_CONFIG.to_string()")]
    config: String,

    /// verbose mode
    #[argh(switch, short = 'v')]
    verbose: bool,
}

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

// Global static variable for arguments
pub static ARGS: Lazy<Mutex<Args>> = Lazy::new(|| {
    let args: Args = argh::from_env();
    Mutex::new(args)
});

pub fn load_args() {
    let args = ARGS.lock().unwrap();
}

fn main() -> io::Result<()> {

    load_args();
    let args = ARGS.lock().unwrap();
    println!("Using config file: {}", args.config);
    if args.verbose {
        println!("Verbose mode is enabled.");
    }
    let now = Local::now();
    let audio_file_name = Arc::new(format!("sine_{}.wav", now.format("%Y-%m-%d_%H:%M:%S%.3f")));
    let mut writer = hound::WavWriter::create(Path::new(&*audio_file_name), spec).unwrap();

    let ram_buffer = Arc::new(Mutex::new(Vec::new()));
    let ram_buffer_clone = Arc::clone(&ram_buffer);

    let port = serialport::new(constants::common::SERIAL_PORT, 
            constants::common::BAUDRATE)
        .timeout(Duration::from_secs(1))
        .open();

    let sync_vec: Vec<u8> = vec![0xFF, 0x01, 0xFF, 0x02, 0xFF, 0x03, 0xFF, 0x04];

    let parser = Arc::new(Mutex::new(parser::parser::Parser::new(sync_vec)));

    // Set a callback to handle parsed frames
    {

        let ram_buffer_for_callback = Arc::clone(&ram_buffer); // Clone for the closure
        let mut parser_lock = parser.lock().unwrap();
        parser_lock.set_callback( move | frame_type, data| {
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
                    let mut buffer = ram_buffer_for_callback.lock().unwrap();
                    let now = Local::now();
                    let frame_number: u32 = (data[4000] as u32)
                    | ((data[4001] as u32) << 8)
                    | ((data[4002] as u32) << 16)
                    | ((data[4003] as u32) << 24);

                    // Convert byte chunks to i16 values
                    let data_i16: Vec<i16> = data.chunks_exact(2) // Process chunks of two bytes
                    .map(|chunk| LittleEndian::read_i16(chunk))
                    .collect(); // Collect into Vec<i16>

                    // Extend the buffer with the converted i16 values
                    buffer.extend_from_slice(&data_i16);
                    println!("{} - AUDIO Frame Received Length: {}, frame_number: {}", now.format("%Y-%m-%d %H:%M:%S%.3f"), data.len(), frame_number); 
                },
            }
        });
    }

    // Start processing thread
    parser::parser::Parser::start(Arc::clone(&parser));

    ctrlc::set_handler(move || {
        println!("\nCtrl+C detected! Flushing buffer to {} file...", audio_file_name);

        // Lock the buffer to access samples
        let buffer = ram_buffer_clone.lock().unwrap();

        for &sample in buffer.iter() {
            writer.write_sample(sample).expect("Failed to write sample");
        }

        writer.flush().expect("Failed to flush writer");

        println!("Audio data written successfully!");
        std::process::exit(0);
    }).expect("Error setting Ctrl+C handler");

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
