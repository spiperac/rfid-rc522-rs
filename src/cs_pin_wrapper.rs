// src/cs_pin_wrapper.rs

use embedded_hal::digital::OutputPin;
use core::convert::Infallible;

pub struct CsPinWrapper<CS> {
    cs: CS,
}

impl<CS> CsPinWrapper<CS>
where
    CS: OutputPin<Error = Infallible>,
{
    pub fn new(cs: CS) -> Self {
        CsPinWrapper { cs }
    }

    pub fn set_high(&mut self) -> Result<(), Infallible> {
        self.cs.set_high()
    }

    pub fn set_low(&mut self) -> Result<(), Infallible> {
        self.cs.set_low()
    }
}
