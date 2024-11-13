#![no_std]
// src/lib.rs

pub mod registers;
pub mod commands;
pub mod rfid_rc522;
pub mod cs_pin_wrapper;

pub use rfid_rc522::RfidRc522;
