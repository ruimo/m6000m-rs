use std::{fmt, sync::mpsc::{self}, thread, time::{Duration, Instant}};
use cpal::traits::HostTrait;
use data_subscriber::{DataSubscriber, StdoutDataSubscriber, VoiceboxDataSubscriber};

use arg::{Args, ArgsErr};
use log::{error, info};
use rodio::{DeviceTrait, Source};
use serial::Port;
use serialport::SerialPort;
use tui::Tui;
use clap::Parser;

mod arg;
mod tui;
mod serial;
mod format;
mod data_subscriber;

#[derive(Debug, PartialEq)]
enum AppErr {
    Aborted,
    NoAvailablePorts,
    SerialPortError(String),
    AudioDeviceError(String)
}

impl fmt::Display for AppErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppErr::Aborted => write!(f, "Quited"),
            AppErr::NoAvailablePorts => write!(f, "No serial ports found in this system. Please confirm the device is connected."),
            AppErr::SerialPortError(msg) => write!(f, "Cannot access serial port: {}", msg),
            AppErr::AudioDeviceError(msg) => write!(f, "Audio device error: {}", msg),
        }
    }
}

fn finalize_args<P: Port>(mut args: Args, tui: &mut Tui, p: &P) -> Result<Args, AppErr> {
    loop {
        match args.error() {
            None => return Ok(args),
            Some(e) => {
                match e {
                    ArgsErr::PortNotSpecified => {
                        match p.available_ports() {
                            Ok(ports) => {
                                if ports.is_empty() {
                                    return Err(AppErr::NoAvailablePorts);
                                }

                                let port = tui.ask_port(ports);
                                if port.is_none() {
                                    return Err(AppErr::Aborted);
                                } else {
                                    args.port = port.map(|p| p.port_name);
                                }
                                return Ok(args);
                            }
                            Err(err) => return Err(AppErr::SerialPortError(err.to_string())),
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(test))]
async fn get_args() -> Result<Args, AppErr> {
    use serial::SerialPort;

    let args: Args = Args::parse();
    let mut tui = Tui {};
    let port = SerialPort {};
    
    finalize_args(args, &mut tui, &port)
}

#[cfg(test)]
async fn get_args() -> Result<Args, AppErr> {
    Ok(Args::parse())
}

fn send_break(ser: &Box<dyn serialport::SerialPort>) {
    match ser.set_break() {
        Ok(_) => {
            thread::sleep(Duration::from_millis(1000));
        }
        Err(err) => {
            error!("Serial port access error. Cannot send break signal {:?}", err);
            std::process::exit(1);
        }
    }

    match ser.clear_break() {
        Ok(_) => {
        }
        Err(err) => {
            error!("Serial port access error. Cannot send break signal {:?}", err);
            thread::sleep(Duration::from_millis(1000));
        }
    }
}

fn send_break_if_needed(ser: &Box<dyn serialport::SerialPort>, last_received: Instant, timeout: Duration) -> bool{
    let elapsed: Duration = last_received.elapsed();
    
    if timeout < elapsed {
        info!("No data comes from the device. Sending break signal...");
        send_break(ser);
        info!("Sending break signal done.");
        true
    } else {
        false
    }
}

fn launch_serialport_worker(mut ser: Box<dyn serialport::SerialPort>, timeout: Duration) -> mpsc::Receiver<Vec<u8>> {
    let (tx, rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = mpsc::channel();
    tokio::spawn(async move {
        let mut last_received_time: Instant = Instant::now();
        let mut buf: [u8; 64] = [0; 64];
        loop {
            match ser.read(&mut buf) {
                Ok(read_size) => {
                    if read_size != 0 {
                        last_received_time = Instant::now();
                        let vec: Vec<u8> = buf[0..read_size].to_vec();
                        match tx.send(vec) {
                            Ok(_) => {}
                            Err(err) => {
                                error!("Fatal error! Failed to communicate worker thread: {:?}", err);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        if send_break_if_needed(&ser, last_received_time, timeout) {
                            last_received_time = Instant::now();
                        }
                    }
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::TimedOut {
                        if send_break_if_needed(&ser, last_received_time, timeout) {
                            last_received_time = Instant::now();
                        }
                    } else {
                        error!("Fatal error! Cannot receive from serial port: {:?}", err);
                        std::process::exit(1);
                    }
                }
            }
        }
    });
    rx
}

fn open_serialport(args: &Args) -> Result<Box<dyn SerialPort>, AppErr> {
    serialport::new(args.port.as_ref().unwrap(), 19200)
    .data_bits(serialport::DataBits::Seven)
    .parity(serialport::Parity::Odd)
    .stop_bits(serialport::StopBits::One)
    .timeout(Duration::from_millis(1000))
    .open()
    .map_err(|e| AppErr::SerialPortError(e.to_string()))
}

fn pick_audio_output_device(args: &Args) -> Result<rodio::Device, AppErr> {
    let host: cpal::Host = cpal::default_host();
    match &args.audio_output_device_name {
        None => return Ok(host.default_output_device().unwrap().into()),
        Some(output_device_name) => {
            let mut devices: Vec<cpal::Device> = host.output_devices().map_err(|e| AppErr::AudioDeviceError(e.to_string()))?.collect();
            if let Some(dev) = devices.iter().find(|d| &d.name().unwrap_or("".to_owned()) == output_device_name) {
                return Ok(dev.clone().into());
            }
            eprintln!("Unknown audio output device name: {}", output_device_name);
            eprintln!("{} audio output device detected:", devices.len());
            for dev in devices {
                eprintln!("  Device '{}'", dev.name().unwrap_or("Unknown".to_owned()));
                match dev.supported_output_configs() {
                    Ok(configs) => {
                        for config in configs {
                            println!("    Configuration:");
                            println!("      Channels: {:?}", config.channels());
                            println!("      Sample rate: {:?}", config.min_sample_rate().0);
                            println!("      Sample format: {:?}", config.sample_format());
                        }
                    },
                    Err(e) => println!("    Cannot retrieve configuration: {}", e),
                }
                eprintln!();
            }
            return Err(AppErr::AudioDeviceError(format!("Device name '{}' is not found.", output_device_name)));
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), AppErr> {
    use std::sync::mpsc;
    use serialport::SerialPort;

    env_logger::init();
    let args = get_args().await?;
    let timeout: Duration = Duration::from_secs(3);
    let ser: Box<dyn SerialPort> = open_serialport(&args)?;
    let rx: mpsc::Receiver<Vec<u8>> = launch_serialport_worker(ser, timeout);
    let mut parser = es51986::parser::Parser::new();
    let mut subscribers: Vec<Box<dyn DataSubscriber>> = vec![Box::new(StdoutDataSubscriber::new(args.output_format.clone()))];
    if let Some(voicebox_url) = &args.voicebox_url {
        let audio_output_device = pick_audio_output_device(&args)?;
        subscribers.push(
            Box::new(VoiceboxDataSubscriber::new(
                voicebox_url.clone(),
                args.voicebox_speaker,
                audio_output_device,
            ))
        )
    }

    loop {
        let received: Vec<u8> = rx.recv().unwrap();
        for r in parser.parse(&received) {
            match r {
                Ok(out) => {
                    for s in subscribers.iter_mut() {
                        s.on_data(&out);
                    }
                }
                Err(err) => error!("Error: {:?}", err),
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serialport::{SerialPortInfo, SerialPortType};
    use crate::{arg::{Args, OutputFormat}, finalize_args, serial::SerialPort, tui::Tui, AppErr};

    #[test]
    fn not_specify_port_no_available_ports() {
        let args: Args = Args { port: None, output_format: OutputFormat::Jsonl, voicebox_url: None, voicebox_speaker: 1, audio_output_device_name: None };
        let mut tui = Tui {
            available_ports: None,
            port_to_return: None,
        };
        let port = SerialPort {
            available_ports: Ok(vec![])
        };
        
        let result: Result<Args, AppErr> = finalize_args(args, &mut tui, &port);
        assert_eq!(result.err().unwrap(), AppErr::NoAvailablePorts);
    }

    #[test]
    fn port_is_specified_by_args() -> Result<(), AppErr> {
        let args: Args = Args { port: Some("Port0".to_owned()), output_format: OutputFormat::Jsonl, voicebox_url: None, voicebox_speaker: 1, audio_output_device_name: None };
        let mut tui = Tui {
            available_ports: None,
            port_to_return: None,
        };
        let port = SerialPort {
            available_ports: Ok(vec![])
        };
        
        let args = finalize_args(args, &mut tui, &port)?;
        assert_eq!(args.port, Some("Port0".to_owned()));
        Ok(())
    }

    #[test]
    fn port_is_not_specified_but_quit() {
        let args: Args = Args { port: None, output_format: OutputFormat::Jsonl, voicebox_url: None, voicebox_speaker: 1, audio_output_device_name: None };
        let mut tui = Tui {
            available_ports: None,
            port_to_return: None,
        };
        let port = SerialPort {
            available_ports: Ok(vec![
                SerialPortInfo {
                    port_name: "port0".to_owned(),
                    port_type: SerialPortType::Unknown,
                }
            ])
        };
        
        let result: Result<Args, AppErr> = finalize_args(args, &mut tui, &port);
        assert_eq!(result.err().unwrap(), AppErr::Aborted);
    }

    #[test]
    fn select_port() -> Result<(), AppErr> {
        let args: Args = Args { port: None, output_format: OutputFormat::Jsonl, voicebox_url: None, voicebox_speaker: 1, audio_output_device_name: None };
        let available_ports = vec![
        SerialPortInfo {
            port_name: "port0".to_owned(),
            port_type: SerialPortType::Unknown,
        },
        SerialPortInfo {
            port_name: "port1".to_owned(),
            port_type: SerialPortType::Unknown,
        },
        ];
        let mut tui = Tui {
            available_ports: Some(available_ports.clone()),
            port_to_return: Some(
                SerialPortInfo {
                    port_name: "port1".to_owned(),
                    port_type: SerialPortType::Unknown,
                }
            )
        };
        let port = SerialPort {
            available_ports: Ok(available_ports.clone())
        };
        
        let args = finalize_args(args, &mut tui, &port)?;
        assert_eq!(args.port, Some("port1".to_owned()));
        Ok(())
    }
}
