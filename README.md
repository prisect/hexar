# hexar
hexagon-shaped radar. 6 antennas, 1 brain. default 24Ghz, maybe will plan for more frequencies
## dependencies
- 6 antennas
- [hexar sdr](https://card.hexar/) card
- usb-to-ttl converter (CP2102, FT232RL)
- UART interface (3.3V / 5V)
- central processing unit/FPGA

## software
linux is required

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

## run
clone, build, run

```
sudo usermod -aG input $(whoami)
# Restart your machine (required)
git clone --recursive https://github.com/prisect/hexar
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Build and run
cargo build --release
cargo run --release
```
