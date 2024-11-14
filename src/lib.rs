#![no_std]
// src/lib.rs

pub mod registers;
pub mod commands;
pub mod rfid_rc522;
pub mod card_types;
pub mod errors;

pub use rfid_rc522::RfidRc522;
