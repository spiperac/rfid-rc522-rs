# RFID_RC522 implementation in Rust 

## Requirements

On Ubuntu, install these requirements:
    sudo apt install avr-libc gcc-avr pkg-config avrdude libudev-dev build-essential

Make sure you have cargo installed and in the path.

Clone git repo and cd into it:
    cd rfid_rc522
    cargo build --release

## Running examples

To run the example, first setup your RAVEDUDE_PORT env variable, point it to where your Arduino device is.
In my case it's /dev/ttyUSB0:
    export RAVEDUDE_PORT=/dev/ttyUSB0

Then run the example:
    cargo run --example dumpinfo

