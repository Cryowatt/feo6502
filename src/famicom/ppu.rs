use crate::devices::BusDevice;

pub struct PPU {}

impl BusDevice for PPU {
    fn read(&self, address: crate::Address) -> Option<u8> {
        todo!()
    }

    fn write(&mut self, address: crate::Address, data: u8) -> bool {
        todo!()
    }
}
