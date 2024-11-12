#![no_std]
#![no_main]

use arduino_hal::prelude::*;
use arduino_hal::spi;
use arduino_hal::default_serial;
use rfid_rc522::registers;
use rfid_rc522::RfidRc522;
use embedded_hal::spi::{Mode, Phase, Polarity};
use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = default_serial!(dp, pins, 9600);

    let settings = spi::Settings {
        data_order: spi::DataOrder::MostSignificantFirst,
        mode: Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        clock: spi::SerialClockRate::OscfOver128,
    };

    let sclk = pins.d13.into_output();
    let mosi = pins.d11.into_output();
    let miso = pins.d12.into_pull_up_input();
    let cs = pins.d10.into_output();
    let (spi, cs_pin) = spi::Spi::new(dp.SPI, sclk, mosi, miso, cs, settings);
    
    let mut rst = pins.d9.into_output(); // Reset pin

    let mut rfid = RfidRc522::new(spi, cs_pin);
    rfid.init(&mut rst, &mut serial); // Pass serial reference to init

    let version = rfid.read_register(&mut serial, registers::VERSION_REG);
    ufmt::uwriteln!(&mut serial, "Direct version read: 0x{:X}", version).unwrap();

    let command_reg = rfid.read_register(&mut serial, registers::COMMAND_REG);
    ufmt::uwriteln!(&mut serial, "COMMAND_REG test read: 0x{:X}", command_reg).unwrap();

    if version == 0x91 || version == 0x92 {
        ufmt::uwriteln!(&mut serial, "MFRC522 communication is OK").unwrap();
    } else {
        ufmt::uwriteln!(&mut serial, "MFRC522 communication error").unwrap();
    }

    loop {}
}
