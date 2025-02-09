use crate::{Address, AddressMask, BusDevice};

use super::rom::{NametableLayout, RomImage};

pub fn mapper_for(rom_image: RomImage) -> impl BusDevice {
    match rom_image.mapper {
        0 => NROM::new(rom_image),
        _ => unimplemented!(),
    }
}

struct NROM {
    prg_map: AddressMask,
    prg_rom: Vec<u8>,
    nametable_layout: NametableLayout,
}

impl NROM {
    pub fn new(rom_image: RomImage) -> Self {
        if rom_image.prg_ram_size > 0 {
            unimplemented!("No PRG RAM support currently");
        }

        let mirror_bits = if rom_image.prg_rom.len() > 16 * 1024 {
            0
        } else {
            1
        };

        Self {
            prg_map: AddressMask::from_block(Address(0x8000), 1, mirror_bits),
            prg_rom: rom_image.prg_rom,
            nametable_layout: rom_image.nametable_layout,
        }
    }
}

impl BusDevice for NROM {
    fn read(&self, address: crate::Address) -> Option<u8> {
        self.prg_map
            .remap(address)
            .map(|prg_address| self.prg_rom[prg_address])
    }

    fn write(&mut self, _address: crate::Address, _data: u8) {}
}
