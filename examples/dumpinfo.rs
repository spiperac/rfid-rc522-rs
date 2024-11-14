#![no_std]
#![no_main]

use arduino_hal::spi;
use arduino_hal::default_serial;
use rfid_rc522::RfidRc522;
use embedded_hal::spi::{Mode, Phase, Polarity};
use panic_halt as _;
use ufmt::uwriteln;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Initialize serial communication
    let mut serial = default_serial!(dp, pins, 9600);

    // Set up SPI communication with specific settings
    let settings = spi::Settings {
        data_order: spi::DataOrder::MostSignificantFirst,
        mode: Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        clock: spi::SerialClockRate::OscfOver64,
    };

    // Set up SPI pins
    let sclk = pins.d13.into_output();
    let mosi = pins.d11.into_output();
    let miso = pins.d12.into_pull_up_input();
    let cs = pins.d10.into_output();
    let (spi, cs_pin) = spi::Spi::new(dp.SPI, sclk, mosi, miso, cs, settings);
    
    // Set up reset pin
    let mut rst = pins.d9.into_output();

    // Initialize the RFID reader
    let mut rfid = RfidRc522::new(spi, cs_pin);
    rfid.init(&mut rst, &mut serial);

    loop {
        // Step 1: Check if a new card is present
        match rfid.is_new_card_present(&mut serial) {
            Ok(true) => {
                uwriteln!(&mut serial, "New card detected.").ok();
            }
            Ok(false) => {
                uwriteln!(&mut serial, "No card detected; retrying...").unwrap();
                arduino_hal::delay_ms(1000);
                continue;
            }
            Err(_) => {
                uwriteln!(&mut serial, "Error checking for new card.").unwrap();
                arduino_hal::delay_ms(1000);
                continue;
            }
        }
        // Delay before the next detection attempt
        arduino_hal::delay_ms(1000);
    }
}
