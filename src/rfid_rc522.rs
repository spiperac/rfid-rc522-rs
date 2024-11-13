use embedded_hal::spi::SpiBus;
use embedded_hal::digital::OutputPin;
use crate::registers::*;
use ufmt::uWrite;

pub struct RfidRc522<SPI, CS> {
    spi: SPI,
    cs: CS,
}

impl<SPI, CS> RfidRc522<SPI, CS>
where
    SPI: SpiBus<u8>,
    CS: OutputPin<Error = core::convert::Infallible>,
{
    pub fn new(spi: SPI, cs: CS) -> Self {
        RfidRc522 { spi, cs }
    }

    pub fn init<W: ufmt::uWrite>(&mut self, reset_pin: &mut dyn OutputPin<Error = core::convert::Infallible>, serial: &mut W) {
        self.cs.set_high().ok();
        ufmt::uwriteln!(serial, "CS set high").ok();
    
        reset_pin.set_low().ok();
        arduino_hal::delay_ms(50);
        reset_pin.set_high().ok();
        arduino_hal::delay_ms(50);
    
        // Soft reset
        self.write_register(serial, COMMAND_REG, 0x0F);
        arduino_hal::delay_ms(50);
    
        let version = self.read_register(serial, VERSION_REG);
        ufmt::uwriteln!(serial, "RFID-RC522 Version: 0x{:X}", version).ok();
    
        // Set up timer and communication parameters
        self.write_register(serial, T_MODE_REG, 0x80);
        self.write_register(serial, T_PRESCALER_REG, 0xA9);
        self.write_register(serial, T_RELOAD_REG_H, 0x03);
        self.write_register(serial, T_RELOAD_REG_L, 0xE8);
    
        self.write_register(serial, TX_ASK_REG, 0x40); // Force 100% ASK modulation
        self.write_register(serial, MODE_REG, 0x3D);   // CRC preset to 0x6363
    
        // Enable the antenna
        self.antenna_on(serial);
    }
    
    pub fn detect_tag<W: uWrite>(&mut self, serial: &mut W) -> Option<[u8; 4]> {
        // Step 1: Clear any pending interrupts and reset FIFO
        self.write_register(serial, COMM_IRQ_REG, 0x7F);
        self.write_register(serial, FIFO_LEVEL_REG, 0x80); // Clear FIFO buffer

        // Step 2: Set BitFramingReg to 0x07 to prepare for REQA
        self.write_register(serial, BIT_FRAMING_REG, 0x07); // Try higher framing
        arduino_hal::delay_ms(5);

        // Step 3: Write REQA command to the FIFO
        self.write_register(serial, FIFO_DATA_REG, REQA);

        // Step 4: Set to TRANSCEIVE mode
        self.write_register(serial, COMMAND_REG, TRANSCEIVE);

        // Step 5: Start transmission
        self.write_register(serial, BIT_FRAMING_REG, 0x87); // Start transmission
        arduino_hal::delay_ms(10);

        // Wait for data in COMM_IRQ_REG or timeout
        let mut timeout = 100;
        while timeout > 0 {
            let irq = self.read_register(serial, COMM_IRQ_REG);
            if irq & 0x30 != 0 {
                ufmt::uwriteln!(serial, "Data available in COMM_IRQ_REG: 0x{:X}", irq).ok();
                break;
            }
            arduino_hal::delay_ms(1);
            timeout -= 1;
        }

        if timeout == 0 {
            // ufmt::uwriteln!(serial, "Timeout waiting for tag after REQA").ok();
            return None;
        }

        // Check FIFO level after REQA
        let fifo_level = self.read_register(serial, FIFO_LEVEL_REG);
        ufmt::uwriteln!(serial, "FIFO level after REQA: {}", fifo_level).ok();

        if fifo_level >= 2 {  // Expecting UID bytes in FIFO after REQA
            let mut uid = [0u8; 4];
            for (i, byte) in uid.iter_mut().enumerate() {
                *byte = self.read_register(serial, FIFO_DATA_REG);
                ufmt::uwriteln!(serial, "UID byte {}: {:02X}", i, *byte).ok();
            }
            return Some(uid);
        }

        // If we only have 2 bytes, try a HALT to reset and possibly reattempt
        if fifo_level == 2 {
            ufmt::uwriteln!(serial, "Partial response detected, trying HALT").ok();

            // Write MIFARE HALT command (0x50)
            self.write_register(serial, FIFO_DATA_REG, 0x50);
            self.write_register(serial, FIFO_DATA_REG, 0x00);
            self.write_register(serial, COMMAND_REG, TRANSCEIVE);

            // Check again after HALT
            arduino_hal::delay_ms(10);
            self.write_register(serial, COMM_IRQ_REG, 0x7F);
            self.write_register(serial, FIFO_LEVEL_REG, 0x80);

            // Retry REQA or return None
            return self.detect_tag(serial);
        }

        // Diagnostic: Output critical register values if REQA fails
        let comm_irq = self.read_register(serial, COMM_IRQ_REG);
        let error_reg = self.read_register(serial, ERROR_REG);
        let fifo_status = self.read_register(serial, FIFO_LEVEL_REG);
        ufmt::uwriteln!(serial, "COMM_IRQ_REG: 0x{:X}, ERROR_REG: 0x{:X}, FIFO_LEVEL_REG: {}", comm_irq, error_reg, fifo_status).ok();

        ufmt::uwriteln!(serial, "No valid UID detected after REQA").ok();
        None
    }        

    fn antenna_on<W: uWrite>(&mut self, serial: &mut W) {
        let current = self.read_register(serial, TX_CONTROL_REG);
        if (current & 0x03) != 0x03 {
            self.write_register(serial, TX_CONTROL_REG, current | 0x03);
        }
    }

    fn write_register<W: uWrite>(&mut self, _serial: &mut W, address: u8, value: u8) {
        let buffer = [address & 0x7F, value];
        let mut read_buffer = [0u8; 2];
        self.cs.set_low().ok();
        self.spi.transfer(&mut read_buffer, &buffer).ok();
        self.cs.set_high().ok();
    }

    fn read_register<W: uWrite>(&mut self, _serial: &mut W, address: u8) -> u8 {
        let buffer = [address | 0x80, 0x00];
        let mut read_buffer = [0u8; 2];
        self.cs.set_low().ok();
        self.spi.transfer(&mut read_buffer, &buffer).ok();
        self.cs.set_high().ok();
        read_buffer[1]
    }

    pub fn check_version_loop<W: uWrite>(&mut self, serial: &mut W) {
        for _ in 0..10 {
            let version = self.read_register(serial, VERSION_REG);
            ufmt::uwriteln!(serial, "Read version: 0x{:X}", version).ok();
            arduino_hal::delay_ms(500);
        }
    }
    
}
