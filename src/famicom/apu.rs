use crate::{devices::BusDevice, Address, AddressMask};

#[derive(Default)]
pub struct Apu {}
impl Apu {
    const ADDRESS_MASK: AddressMask = AddressMask::from_block(Address(0x4000), 11, 0);
}
impl BusDevice for Apu {
    fn read(&mut self, address: Address) -> Option<u8> {
        Self::ADDRESS_MASK
            .remap(address)
            .map(|register| register.0 as u8)
    }

    fn write(&mut self, address: Address, _data: u8) -> bool {
        match Self::ADDRESS_MASK.remap(address) {
            Some(_) => true,
            None => false,
        }
    }
}
