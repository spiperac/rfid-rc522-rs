use core::fmt::{Debug, Formatter, Result};

pub enum CardType {
    Mifare1K,
    Mifare4K,
    MifareUltralight,
    Unknown,
}

impl Debug for CardType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            CardType::Mifare1K => write!(f, "Mifare1K"),
            CardType::Mifare4K => write!(f, "Mifare4K"),
            CardType::MifareUltralight => write!(f, "MifareUltralight"),
            CardType::Unknown => write!(f, "Unknown"),
        }
    }
}
