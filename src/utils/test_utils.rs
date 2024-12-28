use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;

/// Reads a file and returns its contents as a vector of bytes.
pub fn read_file_as_bytes<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}