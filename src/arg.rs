use clap::{arg, command, Parser, ValueEnum};

#[derive(ValueEnum, Debug, PartialEq, Clone)]
pub enum OutputFormat {
    Jsonl,
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// serial port to connect.
    #[arg(long)]
    pub port: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value = "jsonl")]
    pub output_format: OutputFormat,
    // Voicebox URL. If specified, will speak the measured data. (Example: --voice_box_udl http://localhost:50021)
    #[arg(long)]
    pub voicebox_url: Option<String>,
    // Voicebox speaker.
    #[arg(long, default_value = "1")]
    pub voicebox_speaker: usize,
    /// Output audio device name.
    #[arg(long)]
    pub audio_output_device_name: Option<String>,
}

impl Args {
    pub fn error(&self) -> Option<ArgsErr> {
        if self.port.is_none() {
            Some(ArgsErr::PortNotSpecified)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum ArgsErr {
    PortNotSpecified,
}