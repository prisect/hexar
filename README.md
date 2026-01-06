# hexar
hexagon-shaped radar. 6 antennas, 1 brain. default 24Ghz, maybe will plan for more frequencies

## setup
```bash
# Prepare machine
sudo usermod -aG input $(whoami)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Restart your machine
git clone --recursive https://github.com/prisect/hexar
```

```bash
# Build and run
cargo build
cargo run --release
```

## dependencies
- 6 antennas
- hexar sdr card
- USB-to-TTL converter (CP2102, FT232RL)
- UART interface (3.3V / 5V)
- central processing unit/FPGA
- *used LD2412 and LD2450, Hi-Link Electronics, 24GHz mmWave
- (optional) LNA [nooelec SAWBird+GOES L-band](https://amzn.to/41CO8sG) for l-band, see bottom

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

# other stuff

## what about, k-band?

i've also tested something completely different, i might add it or implement it later in some sort.

using the Dish Tailgater’s poorly documented “nudge” command, which moves the antenna in very small steps. each azimuth nudge is about 0.2 degrees, though elevation nudges later proved inconsistent.

<img width="640" height="392" alt="image" src="https://github.com/user-attachments/assets/9e270249-0654-4130-b7d0-014996c2238f" />

i wanted to improve the original low-resolution scan of the clarke belt, where each pixel represented one degree of azimuth and elevation. scanning 180 degrees of southern sky took over three hours, which was the fastest possible due to antenna firmware limitations.

below is a close-up of the inset from the original image, still using the low-resolution scan where each colored square is one degree by one degree.

<img width="867" height="502" alt="image" src="https://github.com/user-attachments/assets/7a55dc80-d899-47af-818e-d23e729a8ef5" />

originally, i scanned in alternating directions to save time, but motor backlash, indexing errors, and inconsistent nudge distances caused the image to drift diagonally. after repeated suggestions, i switched to single-direction scans, sacrificing aesthetics but finally making high-resolution imaging work.

<img width="867" height="502" alt="image" src="https://github.com/user-attachments/assets/2b19abd4-acb2-4395-a609-cadb0920ba37" />

i wasn’t sure the higher resolution would help, since the dish beamwidth exceeds one degree, but the finer 0.2-degree steps produced a clearer image despite some remaining noise and reflections.

## l-band?

here is an diy l-band (~1.7GHz). in addition to the wood pieces, it needs some kind of ground plane (using an aluminum pan lid), some stiff wire (pre-coiled around a 1/5″ PVC pipe), an SMA connector, and an LNA. also might use something like this. but, i like k-band more, it's just, more efficient i would say.

<img width="1873" height="1054" alt="image" src="https://github.com/user-attachments/assets/c48a754c-6e18-4e98-a2c1-b56d6bc2eaf8" />


# great resources
- https://github.com/saveitforparts/Tailgater-Microwave-Imaging
- https://uhf-satcom.com/satellite-reception/uhf

