use serialport::SerialPortInfo;

pub trait Port {
  fn available_ports(&self) -> serialport::Result<Vec<SerialPortInfo>>;
}

#[cfg(not(test))]
pub struct SerialPort {
}

#[cfg(not(test))]
impl Port for SerialPort {
  fn available_ports(&self) -> serialport::Result<Vec<SerialPortInfo>> {
    serialport::available_ports()
  }
}

#[cfg(test)]
pub struct SerialPort {
  pub available_ports: serialport::Result<Vec<SerialPortInfo>>,
}

#[cfg(test)]
impl Port for SerialPort {
  fn available_ports(&self) -> serialport::Result<Vec<SerialPortInfo>> {
    self.available_ports.clone()
  }
}