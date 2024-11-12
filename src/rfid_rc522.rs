use crate::registers;
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
        // Deselect the device by setting CS high
        self.cs.set_high().ok();
        ufmt::uwriteln!(serial, "CS set high").ok();
    
        // Perform a hard reset
        reset_pin.set_low().ok();
        arduino_hal::delay_ms(50);
        reset_pin.set_high().ok();
        arduino_hal::delay_ms(50);
    
        // Perform a soft reset
        self.write_register(serial, COMMAND_REG, 0x0F); // Soft reset command
        arduino_hal::delay_ms(50); // Wait for reset to complete
    
        // Check version after reset
        for _ in 0..3 {
            let version = self.read_register(serial, VERSION_REG);
            ufmt::uwriteln!(serial, "Version after reset attempt: 0x{:X}", version).ok();
            if version != 0x00 {
                break;
            }
            arduino_hal::delay_ms(10);
        }
    
        // Clear interrupt flags and reset FIFO buffer
        self.write_register(serial, COMM_IRQ_REG, 0x7F); // Clear all interrupt request bits
        self.write_register(serial, FIFO_LEVEL_REG, 0x80); // Flush the FIFO buffer
        self.write_register(serial, COMMAND_REG, 0x00);   // Enter idle mode
    
        // Set baud rate and modulation settings
        self.write_register(serial, TX_MODE_REG, 0x00);
        self.write_register(serial, RX_MODE_REG, 0x00);
        self.write_register(serial, MODE_WIDTH_REG, 0x26); // Set modulation width
    
        // Configure timer for communication timeout
        self.write_register(serial, T_MODE_REG, 0x80); // Timer auto-restart
        self.write_register(serial, T_PRESCALER_REG, 0xA9); // Timer prescaler
        self.write_register(serial, T_RELOAD_REG_H, 0x03); // Timer reload high byte
        self.write_register(serial, T_RELOAD_REG_L, 0xE8); // Timer reload low byte
    
        // Configure modulation and transmission settings
        self.write_register(serial, TX_ASK_REG, 0x40); // 100% ASK modulation
        self.write_register(serial, MODE_REG, 0x3D);   // CRC preset to 0x6363
    
        // Turn on the antenna
        self.antenna_on(serial);
    }
    

    fn antenna_on<W: uWrite>(&mut self, serial: &mut W) {
        let current = self.read_register(serial, TX_CONTROL_REG);
        if (current & 0x03) != 0x03 {
            self.write_register(serial, TX_CONTROL_REG, current | 0x03); // Enable TX1 and TX2
        }
    }

    pub fn write_register<W: uWrite>(&mut self, serial: &mut W, address: u8, value: u8) {
        let mut buffer = [address & 0x7F, value]; // Clear MSB for write operation
        let mut read_buffer = [0u8; 2];

        self.cs.set_low().ok(); // Start communication
        let _ = ufmt::uwriteln!(serial, "CS set low");

        // Perform SPI transfer
        let _ = ufmt::uwriteln!(serial, "Writing to address 0x{:X} value 0x{:X}", buffer[0], buffer[1]);
        self.spi.transfer(&mut read_buffer, &buffer).ok();

        // Log read buffer result
        let _ = ufmt::uwriteln!(serial, "SPI write response 0x{:X} 0x{:X}", read_buffer[0], read_buffer[1]);

        self.cs.set_high().ok(); // End communication
        let _ = ufmt::uwriteln!(serial, "CS set high");
    }

    pub fn read_register<W: uWrite>(&mut self, serial: &mut W, address: u8) -> u8 {
        let mut buffer = [address | 0x80, 0x00]; // Set MSB for read operation
        let mut read_buffer = [0u8; 2];

        self.cs.set_low().ok(); // Start communication
        let _ = ufmt::uwriteln!(serial, "CS set low");

        // Perform SPI transfer
        let _ = ufmt::uwriteln!(serial, "Reading from address 0x{:X}", buffer[0]);
        self.spi.transfer(&mut read_buffer, &buffer).ok();

        // Log received data
        let _ = ufmt::uwriteln!(serial, "SPI read response 0x{:X} 0x{:X}", read_buffer[0], read_buffer[1]);

        self.cs.set_high().ok(); // End communication
        let _ = ufmt::uwriteln!(serial, "CS set high");

        read_buffer[1] // Return the register value
    }
}
