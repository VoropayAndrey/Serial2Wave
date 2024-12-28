pub const SERIAL_PORT: &str = "/dev/tty.usbmodem01234567891"; // Change this to match your serial port
pub const SERIAL_READ_SIZE: usize = 8192;
pub const BAUDRATE: u32 = 2_000_000;
pub const PACKET_LENGTH: usize = 4012;
pub const TARGET_SEQUENCE: [u8; 8] = [0xFF, 0x01, 0xFF, 0x02, 0xFF, 0x03, 0xFF, 0x04];
