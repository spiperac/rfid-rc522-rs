#![no_std]
#![no_main]

use arduino_hal::prelude::*;
use arduino_hal::spi;
use arduino_hal::default_serial;
use rfid_rc522::RfidRc522;
use embedded_hal::spi::{Mode, Phase, Polarity};
use panic_halt as _;
use rfid_rc522::card_types::CardType;
use ufmt::uwriteln;
use rfid_rc522::rfid_rc522::RFIDError;

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
        // Attempt to detect the card type
        match rfid.detect_card_type(&mut serial) {
            Ok(Some(card_type)) => {
                match card_type {
                    CardType::Mifare1K => uwriteln!(&mut serial, "Detected card type: Mifare1K").ok(),
                    CardType::Mifare4K => uwriteln!(&mut serial, "Detected card type: Mifare4K").ok(),
                    CardType::MifareUltralight => uwriteln!(&mut serial, "Detected card type: MifareUltralight").ok(),
                    CardType::Unknown => uwriteln!(&mut serial, "Detected card type: Unknown").ok(),
                };

                // After detecting the card type, attempt to retrieve UID with anti-collision
                match rfid.anticoll(&mut serial) {
                    Ok(Some(uid)) => {
                        uwriteln!(&mut serial, "Card UID: {:02X} {:02X} {:02X} {:02X}", uid[0], uid[1], uid[2], uid[3]).ok();
                    }
                    Ok(None) => {
                        uwriteln!(&mut serial, "No UID retrieved; retrying anti-collision...").ok();
                    }
                    Err(_) => {
                        uwriteln!(&mut serial, "Error during anti-collision process").ok();
                    }
                }
            }
            Ok(None) => {
                uwriteln!(&mut serial, "No card detected; retrying...").unwrap();
            }
            Err(e) => {
                match e {
                    RFIDError::CommunicationError => uwriteln!(&mut serial, "Error: CommunicationError").unwrap(),
                    RFIDError::Timeout => uwriteln!(&mut serial, "Error: Timeout").unwrap(),
                    RFIDError::InvalidResponse => uwriteln!(&mut serial, "Error: InvalidResponse").unwrap(),
                };
            }
        }

        // Delay between each detection attempt
        arduino_hal::delay_ms(1000);
    }
}
