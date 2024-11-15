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
        self.set_antenna_gain_max(serial);

    }

    pub fn pcd_calculate_crc<W: ufmt::uWrite>(
        &mut self,
        serial: &mut W,
        data: &[u8],
        crc: &mut [u8; 2],
    ) -> Result<(), RFIDError> {
        // Reset the CRC calculator and configure it
        self.write_register(serial, COMMAND_REG, 0x00); // Set to IDLE state
        self.write_register(serial, DIV_IRQ_REG, 0x04); // Clear CRC interrupt
        self.write_register(serial, FIFO_LEVEL_REG, 0x80); // Flush FIFO
    
        // Write data to FIFO for CRC calculation
        for &byte in data {
            self.write_register(serial, FIFO_DATA_REG, byte);
        }
    
        // Start CRC calculation
        self.write_register(serial, COMMAND_REG, 0x03); // Command: PCD_CALC_CRC
    
        // Wait for the CRC calculation to complete
        let mut timeout = 100;
        while timeout > 0 {
            let irq = self.read_register(serial, DIV_IRQ_REG);
            if irq & 0x04 != 0 {
                break; // CRC calculation complete
            }
            arduino_hal::delay_ms(1);
            timeout -= 1;
        }
    
        if timeout == 0 {
            return Err(RFIDError::Timeout); // Return timeout error if CRC calculation doesn't complete
        }
    
        // Retrieve the CRC result from the CRC_RESULT_REG
        crc[0] = self.read_register(serial, CRC_RESULT_REG_L);
        crc[1] = self.read_register(serial, CRC_RESULT_REG_H);
    
        Ok(())
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
        
        // Reset baud rates
        self.write_register(serial, TX_MODE_REG,0x00);
        self.write_register(serial, RX_MODE_REG, 0x00);  
        // Reset ModWidthReg
        self.write_register(serial, MODE_WIDTH_REG, 0x26);

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

    pub fn read_card_serial<W: ufmt::uWrite>(&mut self, serial: &mut W) -> Result<Option<[u8; 10]>, RFIDError> {
        // Directly attempt card selection, which will handle anti-collision internally
        let mut uid = [0u8; 10]; // UID buffer
        
        // Call select_card, which is equivalent to the C++ `PICC_Select`
        if self.select_card(serial, &mut uid, 0).is_ok() {
            ufmt::uwriteln!(serial, "Card successfully selected. UID:").ok();
            for &byte in &uid {
                ufmt::uwriteln!(serial, "{:02X}", byte).ok();
            }
            return Ok(Some(uid));
        } else {
            ufmt::uwriteln!(serial, "Failed to select card.").ok();
            return Ok(None);
        }
    }

    pub fn select_card<W: ufmt::uWrite>(
        &mut self,
        serial: &mut W,
        uid: &mut [u8; 10],
        valid_bits: u8,
    ) -> Result<u8, RFIDError> {
        let mut cascade_level = 1;
        let mut uid_complete = false;
        let mut current_level_known_bits = valid_bits;
        let mut tx_last_bits: u8 = 0;
        let mut sak: u8 = 0;
        let mut uid_index = 0;
    
        while !uid_complete {
            let (sel_command, uid_start) = match cascade_level {
                1 => (PICC_CMD_SEL_CL1, 0),
                2 => (PICC_CMD_SEL_CL2, 3),
                3 => (PICC_CMD_SEL_CL3, 6),
                _ => return Err(RFIDError::InvalidResponse),
            };
    
            // Prepare buffer
            let mut buffer = [0u8; 9];
            buffer[0] = sel_command;
            let mut index = 2;
    
            // If using Cascade Tag
            if cascade_level > 1 && uid[uid_start - 1] == PICC_CMD_CT {
                buffer[index] = PICC_CMD_CT;
                index += 1;
            }
    
            // Copy UID bytes into buffer
            let bytes_to_copy = current_level_known_bits / 8 + (current_level_known_bits % 8 != 0) as u8;
            for i in 0..bytes_to_copy.min(4 - (index as u8)) {
                buffer[index] = uid[uid_start + i as usize];
                index += 1;
            }
    
            // Set NVB (Number of Valid Bits)
            buffer[1] = ((index as u8) << 4) | tx_last_bits;
    
            // Transceive and handle response
            let mut response_buffer = [0u8; 3];
            let mut response_length = 3; // Ensure this is passed as a mutable reference
            let result = self.transceive(
                serial,
                &buffer[..index as usize + 1],
                &mut response_buffer,
                &mut response_length,
            );
    
            if let Err(err) = result {
                if err == RFIDError::Collision {
                    let coll_pos = self.read_register(serial, COLL_REG) & 0x1F;
                    if coll_pos == 0 {
                        return Err(RFIDError::Collision); // Collision but no valid position
                    }
                    current_level_known_bits = coll_pos;
                } else {
                    return Err(err);
                }
            } else {
                if current_level_known_bits >= 32 {
                    // Check SAK
                    sak = response_buffer[0];
                    if sak & 0x04 != 0 {
                        cascade_level += 1;
                    } else {
                        uid_complete = true;
                    }
                } else {
                    current_level_known_bits = 32; // Continue anti-collision for full UID
                }
            }
        }
    
        Ok(sak)
    }
    
    fn transceive<W: ufmt::uWrite>(
        &mut self,
        serial: &mut W,
        send_buffer: &[u8],
        receive_buffer: &mut [u8],
        receive_length: &mut usize,
    ) -> Result<(), RFIDError> {
        // Write data to FIFO
        for &byte in send_buffer {
            self.write_register(serial, FIFO_DATA_REG, byte);
        }
        // Initiate transceive command
        self.write_register(serial, COMMAND_REG, TRANSCEIVE);
        // Wait for response or timeout
        let mut timeout = 100;
        while timeout > 0 {
            let irq = self.read_register(serial, COMM_IRQ_REG);
            if irq & 0x30 != 0 {
                // Process received data
                let fifo_level = self.read_register(serial, FIFO_LEVEL_REG) as usize;
                for i in 0..fifo_level.min(receive_buffer.len()) {
                    receive_buffer[i] = self.read_register(serial, FIFO_DATA_REG);
                }
                *receive_length = fifo_level;
                return Ok(());
            }
            arduino_hal::delay_ms(1);
            timeout -= 1;
        }
        Err(RFIDError::Timeout)
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

    pub fn set_antenna_gain_max<W: ufmt::uWrite>(&mut self, serial: &mut W) {
        let max_gain = 0x70; // Maximum gain value for the RF_CFG_REG
        self.write_register(serial, RF_CFG_REG, max_gain);
        ufmt::uwriteln!(serial, "Antenna gain set to maximum").ok();
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
