#![no_std]

// src/lib.rs

pub mod registers;
pub mod commands;
pub mod rfid_rc522;
pub mod cs_pin_wrapper;

pub use rfid_rc522::RfidRc522;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
