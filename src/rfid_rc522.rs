use embedded_hal::spi::SpiBus;
use embedded_hal::digital::OutputPin;
use crate::registers::*;
use crate::card_types::CardType; // Import CardType from separate file
use ufmt::uWrite;
use crate::errors::RFIDError;

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
        // Perform a hardware reset
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
    
        // Configure registers
        self.write_register(serial, TX_MODE_REG, 0x00);
        self.write_register(serial, RX_MODE_REG, 0x00);
        self.write_register(serial, MODE_WIDTH_REG, 0x26);
        self.write_register(serial, T_MODE_REG, 0x80);
        self.write_register(serial, T_PRESCALER_REG, 0xA9);
        self.write_register(serial, T_RELOAD_REG_H, 0x03);
        self.write_register(serial, T_RELOAD_REG_L, 0xE8);
        self.write_register(serial, TX_ASK_REG, 0x40); // 100% ASK
        self.write_register(serial, MODE_REG, 0x3D);   // CRC preset to 0x6363
        self.antenna_on(serial); // Enable the antenna
    }
    
    pub fn picc_select<W: ufmt::uWrite>(&mut self, serial: &mut W, uid: &mut [u8; 10], valid_bits: u8) -> Result<(), RFIDError> {
        let mut cascade_level = 1;
        let mut uid_complete = false;
        let mut buffer = [0u8; 9]; // Buffer for SELECT and anti-collision
        let mut current_level_known_bits = 0;
        let mut select_done = false;
    
        while !uid_complete {
            // Set the cascade level
            match cascade_level {
                1 => {
                    buffer[0] = 0x93; // PICC_CMD_SEL_CL1
                    current_level_known_bits = valid_bits;
                }
                2 => {
                    buffer[0] = 0x95; // PICC_CMD_SEL_CL2
                    current_level_known_bits = valid_bits;
                }
                3 => {
                    buffer[0] = 0x97; // PICC_CMD_SEL_CL3
                    current_level_known_bits = valid_bits;
                }
                _ => return Err(RFIDError::InvalidResponse),
            }
    
            let index = 2;
            if current_level_known_bits >= 32 {
                // Full UID is known for this level; send SELECT command with full UID
                buffer[1] = 0x70; // NVB (Number of Valid Bits)
                // Send the buffer for SELECT with BCC (Block Check Character)
                self.write_register(serial, FIFO_DATA_REG, buffer[0]);
                self.write_register(serial, FIFO_DATA_REG, buffer[1]);
                self.write_register(serial, FIFO_DATA_REG, buffer[2]);
                self.write_register(serial, FIFO_DATA_REG, buffer[3]);
            } else {
                // Anticollision: Partially known UID, continue requesting more bits
                buffer[1] = 0x20; // NVB for anti-collision
                self.write_register(serial, FIFO_DATA_REG, buffer[0]);
                self.write_register(serial, FIFO_DATA_REG, buffer[1]);
            }
    
            // Step 2: Send the SELECT command and check for response
            let response = self.read_response(serial)?;
            if response.is_none() {
                return Err(RFIDError::Timeout); // Timeout if no response
            }
    
            // Step 3: Check FIFO level for valid data
            let fifo_level = self.read_register(serial, FIFO_LEVEL_REG);
            if fifo_level >= 4 {
                // We have enough data in FIFO, process the UID bytes
                for i in 0..4 {
                    uid[i] = self.read_register(serial, FIFO_DATA_REG);
                }
    
                // Verify UID is complete (no collision)
                let mut crc = [0u8; 2];
                let result = self.pcd_calculate_crc(&buffer[0..7], &mut crc);
                if result == RFIDError::CommunicationError {
                    return Err(RFIDError::CommunicationError);
                }
    
                // Compare CRC
                if buffer[7] == crc[0] && buffer[8] == crc[1] {
                    select_done = true;
                    uid_complete = true;
                } else {
                    ufmt::uwriteln!(serial, "CRC check failed for UID").ok();
                }
            }
    
            // If collision is detected, adjust and retry
            if select_done {
                cascade_level += 1;
            }
        }
        Ok(())
    }

    fn pcd_calculate_crc(&mut self, data: &[u8], crc: &mut [u8; 2]) -> RFIDError {
        // Initial values for CRC calculation
        let mut crc_reg = 0x6363; // CRC initial value
        let mut current_byte;
        let mut current_bit: u8; // Add explicit type for current_bit
    
        // Process each byte in the data
        for &byte in data {
            current_byte = byte;
            // Process each bit in the byte
            for i in 0..8 {
                let carry = (crc_reg & 0x8000) != 0; // Check the highest bit of CRC
                crc_reg <<= 1; // Shift CRC to the left
    
                // If there was a carry, XOR with the polynomial 0x1021
                if carry ^ ((current_byte >> (7 - i)) & 0x01 != 0) {
                    crc_reg ^= 0x1021; // Polynomial 0x1021
                }
            }
        }
    
        // Store the CRC result
        crc[0] = (crc_reg >> 8) as u8; // High byte
        crc[1] = (crc_reg & 0xFF) as u8; // Low byte
    
        // If everything went well, return success
        RFIDError::CommunicationError
    }
    
    
    pub fn detect_card_type<W: ufmt::uWrite>(&mut self, serial: &mut W) -> Result<Option<CardType>, RFIDError> {
        // Clear any pending interrupts and reset FIFO
        self.write_register(serial, COMM_IRQ_REG, 0x7F);
        self.write_register(serial, FIFO_LEVEL_REG, 0x80); // Clear FIFO buffer
    
        // Send the REQA command to check for a card
        let reqa_command = 0x26;
        self.send_command(serial, reqa_command)?;
    
        // Wait for a response
        let response = self.read_response(serial)?;
        if response.is_none() {
            return Ok(None); // No card detected if no response
        }
    
        // Check FIFO level to see if we received a valid response
        let fifo_level = self.read_register(serial, FIFO_LEVEL_REG);
        if fifo_level < 2 {
            return Ok(None); // No valid response, so no card detected
        }
    
        // Read the SAK (Select Acknowledge) from the FIFO
        let sak = self.get_sak(serial)?;
    
        // Determine the card type based on SAK
        let card_type = match sak {
            0x04 => CardType::Mifare1K,
            0x08 => CardType::Mifare4K,
            0x00 => CardType::MifareUltralight,
            _ => CardType::Unknown,
        };
    
        // Only return a detected card type if SAK is valid and meaningful
        if sak != 0x00 && sak != 0xFF {
            Ok(Some(card_type))
        } else {
            Ok(None) // No valid card detected
        }
    }

    pub fn is_new_card_present<W: ufmt::uWrite>(&mut self, serial: &mut W) -> Result<bool, RFIDError> {
        // Clear any pending interrupts and reset FIFO
        self.write_register(serial, COMM_IRQ_REG, 0x7F);
        self.write_register(serial, FIFO_LEVEL_REG, 0x80); // Clear FIFO buffer
    
        // Send the REQA command to check for a card
        //let reqa_command = 0x26;
        //self.send_command(serial, reqa_command)?;
        self.request_a(serial);
        
        // Wait for a response
        let response = self.read_response(serial)?;
        if response.is_none() {
            return Ok(false); // No card detected if no response
        }
    
        // Card detected based on FIFO level response
        Ok(true)
    }

    // Correct implementation of REQA or WUPA as per MFRC522 library (with minimal changes)
    pub fn picc_reqa_or_wupa<W: ufmt::uWrite>(
        &mut self,
        serial: &mut W,
        command: u8, // 0x26 for REQA, 0x52 for WUPA
        buffer: &mut [u8; 2], // Buffer to store ATQA response
        buffer_size: &mut u8,  // Buffer size (should be at least 2 bytes)
    ) -> Result<(), RFIDError> {
        let valid_bits = 7; // REQA/WUPA only requires 7 bits for the last byte (short frame format)

        // Ensure the buffer has space for ATQA (2 bytes)
        if buffer.is_empty() || *buffer_size < 2 {
            return Err(RFIDError::InvalidResponse); // ATQA must be 2 bytes
        }

        // Send REQA or WUPA command using the FIFO (this is the correct approach as in their code)
        self.send_command(serial, command);

        // Wait for the response
        let response = self.read_response(serial)?;
        if response.is_none() {
            return Err(RFIDError::Timeout); // Timeout if no response
        }

        // Read the ATQA response (should be exactly 2 bytes)
        let fifo_level = self.read_register(serial, FIFO_LEVEL_REG);
        if fifo_level >= 2 {
            for i in 0..2 {
                buffer[i] = self.read_register(serial, FIFO_DATA_REG);
            }
        } else {
            return Err(RFIDError::Timeout); // No valid response in FIFO
        }

        // Check if the ATQA response is valid (not zero)
        if buffer[0] == 0x00 || buffer[1] == 0x00 {
            return Err(RFIDError::InvalidResponse); // Invalid ATQA response
        }

        // Set buffer size to 2 (as expected)
        *buffer_size = 2;

        Ok(())
    }

    // Refactored request_a to use picc_reqa_or_wupa
    pub fn request_a<W: ufmt::uWrite>(&mut self, serial: &mut W) -> Result<bool, RFIDError> {
        let mut buffer = [0u8; 2];
        let mut buffer_size = 2;

        // Use picc_reqa_or_wupa to send the REQA command
        self.picc_reqa_or_wupa(serial, 0x26, &mut buffer, &mut buffer_size)?;

        // Check if the response is valid (ATQA is not zero)
        Ok(buffer[0] != 0x00 && buffer[1] != 0x00)
    }

    pub fn anticoll<W: ufmt::uWrite>(&mut self, serial: &mut W) -> Result<Option<[u8; 4]>, RFIDError> {
        // Step 1: Reset any previous FIFO data
        self.write_register(serial, COMM_IRQ_REG, 0x7F);
        self.write_register(serial, FIFO_LEVEL_REG, 0x80); // Clear FIFO buffer
    
        // Step 2: Send the WUPA (Wake-up) command to the card, which might be required for certain cards
        let wupa_command = 0x52; // WUPA (Wake Up A) command
        self.send_command(serial, wupa_command)?;
    
        // Step 3: Wait for a response to the WUPA command
        let response = self.read_response(serial)?;
        if response.is_none() {
            ufmt::uwriteln!(serial, "Timeout waiting for WUPA response.").ok();
            return Ok(None); // Timeout or no response
        }
    
        // Step 4: Send the Anti-collision command (0x93 for Mifare Classic)
        let anticoll_command = 0x93;
        self.send_command(serial, anticoll_command)?;
    
        // Step 5: Wait for the anti-collision response and check if it was successful
        let mut timeout = 200; // Increased timeout for anti-collision
        while timeout > 0 {
            let irq = self.read_register(serial, COMM_IRQ_REG);
            if irq & 0x30 != 0 {
                ufmt::uwriteln!(serial, "Response detected in COMM_IRQ_REG: 0x{:X}", irq).ok();
                break;
            }
            arduino_hal::delay_ms(5); // Small delay for processing
            timeout -= 1;
        }
    
        if timeout == 0 {
            ufmt::uwriteln!(serial, "Timeout waiting for anti-collision response.").ok();
            return Ok(None); // Timeout or no response
        }
    
        // Step 6: Check the FIFO level for valid data
        let fifo_level = self.read_register(serial, FIFO_LEVEL_REG);
        ufmt::uwriteln!(serial, "FIFO level after anti-collision: {}", fifo_level).ok();
        
        if fifo_level >= 4 {
            // Read the UID from FIFO if there are enough bytes
            let mut uid = [0u8; 4];
            for (i, byte) in uid.iter_mut().enumerate() {
                *byte = self.read_register(serial, FIFO_DATA_REG);
                ufmt::uwriteln!(serial, "UID byte {}: {:02X}", i, *byte).ok();
            }
            return Ok(Some(uid)); // Successfully retrieved UID
        } else {
            ufmt::uwriteln!(serial, "Not enough data in FIFO after anti-collision. FIFO level: {}", fifo_level).ok();
        }
    
        // If FIFO doesn't have enough data, return None
        ufmt::uwriteln!(serial, "Not enough data in FIFO after anti-collision.").ok();
        Ok(None)
    }
    
    
    pub fn read_card_serial<W: ufmt::uWrite>(&mut self, serial: &mut W) -> Result<Option<[u8; 4]>, RFIDError> {
        // Ensure that a card is present
        if !self.is_new_card_present(serial)? {
            return Ok(None);
        }

        // Attempt anti-collision to retrieve the UID
        let uid = self.anticoll(serial)?;

        // If we have a UID, perform card selection
        if let Some(uid) = uid {
            if self.select_card(serial, &uid)? {
                Ok(Some(uid))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn select_card<W: ufmt::uWrite>(&mut self, serial: &mut W, uid: &[u8; 4]) -> Result<bool, RFIDError> {
        // Clear interrupts
        self.write_register(serial, COMM_IRQ_REG, 0x7F);
        self.write_register(serial, FIFO_LEVEL_REG, 0x80); // Clear FIFO buffer

        // Send the Select command and UID
        self.write_register(serial, FIFO_DATA_REG, 0x93); // Select command for Cascade Level 1
        self.write_register(serial, FIFO_DATA_REG, 0x70); // NVB - Number of Valid Bits

        // Write UID bytes to FIFO
        for &byte in uid {
            self.write_register(serial, FIFO_DATA_REG, byte);
        }

        // Calculate and write BCC (Block Check Character)
        let bcc = uid[0] ^ uid[1] ^ uid[2] ^ uid[3];
        self.write_register(serial, FIFO_DATA_REG, bcc);

        // Set command to TRANSCEIVE
        self.write_register(serial, COMMAND_REG, TRANSCEIVE);
        self.write_register(serial, BIT_FRAMING_REG, 0x00);

        // Wait for a response
        let mut timeout = 100;
        while timeout > 0 {
            let irq = self.read_register(serial, COMM_IRQ_REG);
            if irq & 0x30 != 0 {
                break; // Response received
            }
            arduino_hal::delay_ms(1);
            timeout -= 1;
        }

        if timeout == 0 {
            return Ok(false); // Timeout, no selection
        }

        // Check for successful selection
        let fifo_level = self.read_register(serial, FIFO_LEVEL_REG);
        Ok(fifo_level >= 1) // SAK (Select Acknowledge) response expected
    }

    fn send_command<W: ufmt::uWrite>(&mut self, serial: &mut W, command: u8) -> Result<(), RFIDError> {
        // Write the command to FIFO and set TRANSCEIVE mode
        self.write_register(serial, FIFO_DATA_REG, command);
        self.write_register(serial, COMMAND_REG, TRANSCEIVE);
        self.write_register(serial, BIT_FRAMING_REG, 0x87); // Start transmission
        Ok(())
    }

    fn read_response<W: ufmt::uWrite>(&mut self, serial: &mut W) -> Result<Option<u8>, RFIDError> {
        let mut timeout = 100;
        while timeout > 0 {
            let irq = self.read_register(serial, COMM_IRQ_REG);
            if irq & 0x30 != 0 {
                return Ok(Some(irq));
            }
            arduino_hal::delay_ms(1);
            timeout -= 1;
        }
        Ok(None) // Timeout if no response
    }

    fn get_sak<W: ufmt::uWrite>(&mut self, serial: &mut W) -> Result<u8, RFIDError> {
        // Logic to communicate and read the SAK byte from the card
        Ok(self.read_register(serial, FIFO_DATA_REG)) // Replace with actual SAK read logic
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

}
