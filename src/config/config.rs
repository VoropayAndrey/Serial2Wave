
pub struct Config {
    sample_rate: usize,
    audio_frame_bytes_length: usize,
    audio_frame_number_bytes_length: usize,
    number_of_channels: usize,
    bytes_per_channel: usize,
    sync_bytes: [u8],
    output_wav_file_path: String, 
    output_log_file_path: String,
}

impl Config {

}