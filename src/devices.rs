use crate::{Address, AddressMask};

pub trait BusDevice {
    fn read(&self, address: Address) -> Option<u8>;
    fn write(&mut self, address: Address, data: u8) -> bool;
}

pub struct RamBank<const SIZE: usize> {
    map: AddressMask,
    memory: [u8; SIZE],
}

impl<const SIZE: usize> RamBank<SIZE> {
    pub fn new(map: AddressMask) -> Self {
        Self {
            map,
            memory: [0u8; SIZE],
        }
    }
}

impl<const SIZE: usize> BusDevice for RamBank<SIZE> {
    fn read(&self, address: Address) -> Option<u8> {
        self.map
            .remap(address)
            .map(|ram_address| self.memory[ram_address])
    }

    fn write(&mut self, address: Address, data: u8) -> bool {
        if let Some(ram_address) = self.map.remap(address) {
            self.memory[ram_address] = data;
            true
        } else {
            false
        }
    }
}
