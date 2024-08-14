#[cfg(not(test))]
use std::io;

use serialport::SerialPortInfo;

#[cfg(not(test))]
pub struct Tui {
}

#[cfg(test)]
pub struct Tui {
  pub available_ports: Option<Vec<SerialPortInfo>>,
  pub port_to_return: Option<SerialPortInfo>,
}

#[cfg(not(test))]
impl Tui {
    pub fn ask_port(&mut self, available_ports: Vec<SerialPortInfo>) -> Option<SerialPortInfo> {
        loop {
            let mut line_buf = String::new();
            eprintln!("Select serial port to connect:");
            for (idx, p) in available_ports.iter().enumerate() {
                eprintln!("{}) {}", idx + 1, p.port_name);
            }
            eprint!("Enter number or 'q' to quit: ");
            io::stdin().read_line(&mut line_buf).unwrap();
            
            let inp = line_buf.trim();
            if inp == "q" {
                return None;
            } else {
                match inp.parse::<usize>() {
                    Ok(n) => {
                        if n < 1 || available_ports.len() < n {
                            if available_ports.len() == 1 {
                                eprintln!("Invalid selection. Please select a number 1 or q: quit");
                            } else {
                                eprintln!("Invalid selection. Please select a number between 1 and {}", available_ports.len());
                            }
                            eprintln!("");
                        } else {
                            return Some(available_ports[n - 1].clone());
                        }
                    }
                    Err(err) => {
                      eprintln!("Error: invalid input. Input number or q: quit. {:?}", err);
                      eprintln!("");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
impl Tui {
    pub fn ask_port(&mut self, available_ports: Vec<SerialPortInfo>) -> Option<SerialPortInfo> {
      self.available_ports = Some(available_ports.clone());
      return self.port_to_return.clone();
    }
}
