# hexar
hexagon-shaped radar. 6 antennas, 1 brain. default 24Ghz, maybe will plan for more frequencies

## setup
```bash
# Prepare machine
sudo usermod -aG input $(whoami)
# Restart your machine (required)
git clone --recursive https://github.com/prisect/hexar
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```bash
# Build and run
cargo build
RUST_LOG=trace cargo run --example main
```

> [!NOTE]
> Software runs on sensitive peices of hardware.
> Run at your own risk :)

## dependencies
- 6 antennas
- [hexar sdr](https://card.hexar/) card
- USB-to-TTL converter (CP2102, FT232RL)
- UART interface (3.3V / 5V)
- central processing unit/FPGA
- *used LD2412 and LD2450, Hi-Link Electronics, 24GHz mmWave

## software
### dev
- rust tc
- cargo
- gcc
- make
- cmake

### py
- pyserial >= 3.5
- numpy >= 1.21
- matplotlib >= 3.5
- scipy >= 1.8
- pyqt5 >= 5.15

### ros2
- rclcpp
- std_msgs
- sensor_msgs
- geometry_msgs
- nav_msgs
- tf2_ros
- rosbag2
- rviz2
## schematic
<img width="822" height="741" alt="Untitled Diagram drawio" src="https://github.com/user-attachments/assets/cb502d66-dc16-40f6-8098-a0f696b19433" />

## LD24
LD2412 and LD2450 driver, made in rust.

Example plot of LD2412:
<br>
<br>
<img src="https://github.com/prisect/hexar/blob/main/out.png">
