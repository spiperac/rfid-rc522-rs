use core::fmt::{Debug, Formatter, Result};
use ufmt::{uDebug, uWrite};

#[derive(PartialEq)]
pub enum RFIDError {
    CommunicationError,
    Timeout,
    InvalidResponse,
    Error,         // New Error variant
    CrcError,      // New CrcError variant
    NoRoom,
    Collision,
}

impl Debug for RFIDError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            RFIDError::CommunicationError => write!(f, "CommunicationError"),
            RFIDError::Timeout => write!(f, "Timeout"),
            RFIDError::InvalidResponse => write!(f, "InvalidResponse"),
            RFIDError::Error => write!(f, "Error"),
            RFIDError::CrcError => write!(f, "CrcError"),
            RFIDError::NoRoom => write!(f, "No room or we"),
            RFIDError::Collision => write!(f, "Collision i guess"),
            
        }
    }
}

// Implementing uDebug for RFIDError
impl uDebug for RFIDError {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<W>) -> core::result::Result<(), W::Error>
    where
        W: uWrite + ?Sized,
    {
        match self {
            RFIDError::CommunicationError => f.write_str("CommunicationError"),
            RFIDError::Timeout => f.write_str("Timeout"),
            RFIDError::InvalidResponse => f.write_str("InvalidResponse"),
            RFIDError::Error => f.write_str("Error"),
            RFIDError::CrcError => f.write_str("CrcError"),
            RFIDError::NoRoom => f.write_str("No room or we"),
            RFIDError::Collision => f.write_str("Collision i guess"),
        }
    }
}
