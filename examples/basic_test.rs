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
        clock: spi::SerialClockRate::OscfOver64,
    };

    let sclk = pins.d13.into_output();
    let mosi = pins.d11.into_output();
    let miso = pins.d12.into_pull_up_input();
    let cs = pins.d10.into_output();
    let (spi, cs_pin) = spi::Spi::new(dp.SPI, sclk, mosi, miso, cs, settings);
    
    let mut rst = pins.d9.into_output(); // Reset pin

    let mut rfid = RfidRc522::new(spi, cs_pin);
    rfid.init(&mut rst, &mut serial); // Pass serial reference to init

    // rfid.check_version_loop(&mut serial);

    loop {
        match rfid.detect_tag(&mut serial) {
            Some(uid) => {
                ufmt::uwriteln!(&mut serial, "Tag detected with UID: ").ok();
                for byte in &uid {
                    ufmt::uwriteln!(&mut serial, "{:02X} ", *byte).ok();
                }
                ufmt::uwriteln!(&mut serial, "").ok(); // Newline
            }
            None => {
            }
        }

        arduino_hal::delay_ms(1000); // Delay between each detection attempt
    }
}
