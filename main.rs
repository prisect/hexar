use anyhow::{Context, Result};
use rosrust::{api::raii::Publisher, init, is_ok, rate, RosHandle};
use serialport::{SerialPort, SerialPortType};
use std::{
    io::Read,
    time::Duration,
};

struct RadarPublisher {
    _ros_handle: RosHandle,
    range_publisher: Publisher<std_msgs::Float32>,
    speed_publisher: Publisher<std_msgs::Float32>,
    serial_port: Box<dyn SerialPort>,
}

impl RadarPublisher {
    fn new(serial_port_path: &str) -> Result<Self> {
        // Initialize ROS node
        rosrust::init("radar_publisher");
        let _ros_handle = RosHandle::new().context("Failed to create ROS handle")?;
        
        rosrust::ros_info!("\u{1b}[1;32m---->\u{1b}[0m Radar Parser Started.");

        // Create publishers
        let range_publisher = rosrust::publish("radar_range", 10)
            .context("Failed to create range publisher")?;
        let speed_publisher = rosrust::publish("radar_speed", 10)
            .context("Failed to create speed publisher")?;

        // Open serial port
        let serial_port = serialport::new(serial_port_path, 9600)
            .timeout(Duration::from_millis(100))
            .open()
            .with_context(|| format!("Failed to open serial port: {}", serial_port_path))?;

        rosrust::ros_debug!("Radar port opened successfully!");

        Ok(Self {
            _ros_handle,
            range_publisher,
            speed_publisher,
            serial_port,
        })
    }

    fn process_buffer(&self, buffer: &[u8], bytes_read: usize) -> Result<()> {
        if bytes_read <= 4 {
            return Ok(());
        }

        // Convert buffer to string for parsing
        let data = String::from_utf8_lossy(&buffer[..bytes_read]);

        if buffer[1] == b'm' && buffer[2] == b'p' && buffer[3] == b's' {
            // Speed message format: "mps X.XX"
            if let Some(value_str) = data.get(6..bytes_read) {
                let speed_value: f32 = value_str.trim().parse()
                    .context("Failed to parse speed value")?;
                
                let speed_msg = std_msgs::Float32 { data: speed_value };
                self.speed_publisher.send(speed_msg)
                    .context("Failed to publish speed message")?;
            }
        } else if buffer[0] == b'\"' && buffer[1] == b'm' && buffer[2] == b'\"' {
            // Range message format: "\"m\" X.XX"
            if let Some(value_str) = data.get(4..bytes_read) {
                let range_value: f32 = value_str.trim().parse()
                    .context("Failed to parse range value")?;
                
                let range_msg = std_msgs::Float32 { data: range_value };
                self.range_publisher.send(range_msg)
                    .context("Failed to publish range message")?;
            }
        }

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        let mut rate = rosrust::rate(100.0)?;
        let mut buffer = vec![0u8; 125];

        while is_ok() {
            // Clear serial buffer
            self.serial_port.clear(serialport::ClearBuffer::Input)?;

            // Read from serial port
            match self.serial_port.read(&mut buffer) {
                Ok(bytes_read) => {
                    if let Err(e) = self.process_buffer(&buffer, bytes_read) {
                        rosrust::ros_warn!("Error processing buffer: {}", e);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // Timeout is expected, continue
                }
                Err(e) => {
                    rosrust::ros_error!("Serial port read error: {}", e);
                    break;
                }
            }

            rate.sleep();
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    // Get serial port parameter with default
    let serial_port = rosrust::param("~serialPort")
        .unwrap_or_else(|| "/dev/ttyACM0".to_string());

    rosrust::ros_info!("Using serial port: {}", serial_port);

    let mut radar_publisher = RadarPublisher::new(&serial_port)?;
    
    if let Err(e) = radar_publisher.run() {
        rosrust::ros_error!("Radar publisher error: {}", e);
        return Err(e);
    }

    rosrust::ros_info!("Radar publisher shutting down.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_speed_message() {
        let mock_ros_handle = RosHandle::new().unwrap();
        let range_pub = rosrust::publish("test_range", 10).unwrap();
        let speed_pub = rosrust::publish("test_speed", 10).unwrap();
        
        // This is a simplified test - in practice you'd use a mock serial port
        let publisher = RadarPublisher {
            _ros_handle: mock_ros_handle,
            range_publisher: range_pub,
            speed_publisher: speed_pub,
            serial_port: serialport::new("/dev/null", 9600).open().unwrap(),
        };

        // Test speed message parsing
        let speed_data = b" mps 12.34";
        assert!(publisher.process_buffer(speed_data, speed_data.len()).is_ok());
        
        // Test range message parsing  
        let range_data = b"\"m\" 5.67";
        assert!(publisher.process_buffer(range_data, range_data.len()).is_ok());
    }
}
