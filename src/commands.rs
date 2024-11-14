// src/commands.rs

// Commands for the MFRC522
pub const PCD_IDLE: u8 = 0x00;
pub const PCD_AUTH: u8 = 0x0E;
pub const PCD_TRANSCEIVE: u8 = 0x0C;

// Add other relevant commands

pub const PCD_CALC_CRC: u8 = 0x03; // CRC calculation command
pub const PCD_RESETPHASE: u8 = 0x0F;
