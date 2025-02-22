use crate::{Address, AddressMask, BusDevice};

use super::rom::{NametableLayout, RomImage};
use crate::ByteUnits as _;

pub fn mapper_from(rom_image: &RomImage) -> (impl BusDevice, impl BusDevice) {
    match rom_image.mapper {
        0 => (NromPrgMapper::new(rom_image), NromChrMapper::new(rom_image)),
        _ => unimplemented!(),
    }
}

pub struct NromPrgMapper {
    prg_ram_map: Option<AddressMask>,
    prg_ram: Vec<u8>,
    prg_rom_map: AddressMask,
    prg_rom: Vec<u8>,
    nametable_layout: NametableLayout,
}

impl NromPrgMapper {
    pub fn new(rom_image: &RomImage) -> Self {
        if rom_image.prg_ram_size > 0 {
            unimplemented!("No PRG RAM support currently");
        }

        let mirror_bits = if rom_image.prg_rom.len() > 16.KiB() {
            0
        } else {
            1
        };

        Self {
            prg_ram_map: None,
            prg_ram: vec![],
            prg_rom_map: AddressMask::from_block(Address(0x8000), 1, mirror_bits),
            prg_rom: rom_image.prg_rom.clone(),
            nametable_layout: rom_image.nametable_layout,
        }
    }

    pub fn new_with_ram(rom_image: &RomImage) -> Self {
        let mirror_bits = if rom_image.prg_rom.len() > 16.KiB() {
            0
        } else {
            1
        };

        Self {
            prg_ram_map: Some(AddressMask::from_block(Address(0x6000), 3, 0)),
            prg_ram: vec![0u8; 8.KiB()],
            prg_rom_map: AddressMask::from_block(Address(0x8000), 1, mirror_bits),
            prg_rom: rom_image.prg_rom.clone(),
            nametable_layout: rom_image.nametable_layout,
        }
    }
}

impl BusDevice for NromPrgMapper {
    #[inline]
    fn read(&mut self, address: crate::Address) -> Option<u8> {
        self.prg_rom_map
            .remap(address)
            .map(|prg_address| self.prg_rom[prg_address])
            .or_else(|| {
                self.prg_ram_map.and_then(|mask| {
                    mask.remap(address)
                        .map(|prg_address| self.prg_ram[prg_address])
                })
            })
    }

    #[inline]
    fn write(&mut self, address: crate::Address, data: u8) -> bool {
        if let Some(ram_offset) = self.prg_ram_map.and_then(|mask| mask.remap(address)) {
            println!("#{:02X} => {:?}", data, address);
            self.prg_ram[ram_offset] = data;
            true
        } else {
            false
        }
    }
}

pub struct NromChrMapper {
    chr_rom: [u8; 8 * usize::K],
    chr_rom_mask: AddressMask,
}

impl NromChrMapper {
    pub fn new(rom_image: &RomImage) -> Self {
        assert_eq!(
            rom_image.chr_rom.len(),
            8.KiB(),
            "NROM CHR ROM must be 8KiB"
        );
        Self {
            chr_rom: rom_image
                .chr_rom
                .clone()
                .try_into()
                .expect("CHR is 8KiB for NROM"),
            chr_rom_mask: AddressMask::from_block(Address(0), 3, 0),
        }
    }
}

impl BusDevice for NromChrMapper {
    #[inline]
    fn read(&mut self, address: crate::Address) -> Option<u8> {
        self.chr_rom_mask
            .remap(address)
            .map(|chr_address| self.chr_rom[chr_address])
    }

    #[inline]
    fn write(&mut self, address: crate::Address, _: u8) -> bool {
        self.chr_rom_mask.remap(address).is_some()
    }
}
