use bitfields::bitfield;
use strum::FromRepr;

use crate::{
    devices::{BusDevice, RamBank},
    macros::from_bits,
    Address, AddressMask,
};

use crate::ByteUnits as _;

#[bitfield(u8)]
struct ControlFlags {
    #[bits(2)]
    nametable_bank: u8,

    #[bits(1)]
    increment_mode: IncrementMode,

    #[bits(1)]
    sprite_pattern_bank: u8,

    #[bits(1)]
    background_pattern_bank: u8,

    #[bits(1)]
    sprite_size: SpriteSize,

    #[bits(1)]
    ext_write_mode: bool,

    #[bits(1)]
    vblank_nmi_enable: bool,
}

#[repr(u8)]
#[derive(FromRepr, Clone, Copy, Debug)]
enum IncrementMode {
    Horizontal = 0,
    Vertical = 1,
}
from_bits!(IncrementMode, u8);

#[repr(u8)]
#[derive(FromRepr, Clone, Copy, Debug)]
enum SpriteSize {
    Size8x8 = 0,
    Size8x16 = 1,
}
from_bits!(SpriteSize, u8);

#[bitfield(u8)]
struct MaskFlags {
    #[bits(1)]
    greyscale_enable: bool,

    #[bits(1)]
    background_overscan: bool,

    #[bits(1)]
    sprite_overscan: bool,

    #[bits(1)]
    render_background: bool,

    #[bits(1)]
    render_sprite: bool,

    #[bits(1)]
    red_emphasize: bool,

    #[bits(1)]
    green_emphasize: bool,

    #[bits(1)]
    blue_emphasize: bool,
}

#[repr(u8)]
#[derive(Default, FromRepr, Clone, Copy)]
enum StatusFlags {
    #[default]
    Default = 0,
    SpriteOverflow = 0b0010_0000,
    Sprite0Hit = 0b0100_0000,
    VBlankFlag = 0b1000_0000,
}

pub struct Ppu<Mapper: BusDevice> {
    control_flags: ControlFlags,
    mask_flags: MaskFlags,
    status: StatusFlags,
    data_latch: u8,
    oam_address: u8,
    oam: [u8; 256],
    scroll_x: u16,
    scroll_y: u16,
    bus_address: Address,
    bus: PpuBus<Mapper>,
    write_swap: bool,
}

impl<Mapper: BusDevice> Ppu<Mapper> {
    const ADDRESS_MASK: AddressMask = AddressMask::from_block(Address(0x2000), 3, 10);

    pub fn new(mapper: Mapper) -> Self {
        Self {
            control_flags: Default::default(),
            mask_flags: Default::default(),
            status: Default::default(),
            data_latch: Default::default(),
            oam_address: Default::default(),
            oam: [0u8; 256],
            scroll_x: Default::default(),
            scroll_y: Default::default(),
            bus_address: Default::default(),
            bus: PpuBus::new(mapper),
            write_swap: Default::default(),
        }
    }

    fn ctrl(&mut self, data: u8) {
        self.control_flags = ControlFlags::from_bits(data);
    }

    fn mask(&mut self, data: u8) {
        self.mask_flags = MaskFlags::from_bits(data);
    }

    fn status(&mut self) -> u8 {
        self.write_swap = false;
        (self.data_latch & 0b0001_1111) | (self.status as u8)
    }

    fn read_oam(&self) -> u8 {
        self.oam[self.oam_address as usize]
    }

    fn write_oam(&mut self, data: u8) {
        // Todo: Writes to OAMDATA during rendering (on the pre-render line and the visible lines 0–239, provided
        // either sprite or background rendering is enabled) do not modify values in OAM, but do perform a glitchy
        // increment of OAMADDR, bumping only the high 6 bits (i.e., it bumps the [n] value in PPU sprite evaluation –
        // it's plausible that it could bump the low bits instead depending on the current status of sprite
        // evaluation). This extends to DMA transfers via OAMDMA, since that uses writes to $2004. For emulation
        // purposes, it is probably best to completely ignore writes during rendering.
        self.oam[self.oam_address as usize] = data;
        self.oam_address = self.oam_address.wrapping_add(1);
    }

    fn scroll(&mut self, data: u8) {
        let nametable = self.control_flags.nametable_bank();
        match self.write_swap {
            false => self.scroll_x = data as u16 | (nametable as u16 & 0b01) << 8,
            true => self.scroll_y = data as u16 | (nametable as u16 & 0b10) << 7,
        }
        self.write_swap = !self.write_swap;
    }

    fn addr(&mut self, data: u8) {
        match self.write_swap {
            false => self.bus_address.set_high(data),
            true => self.bus_address.set_low(data),
        }
        self.write_swap = !self.write_swap;
    }

    fn read_vram(&mut self) -> u8 {
        self.bus.read(self.bus_address).unwrap()
    }

    fn write_vram(&mut self, data: u8) {
        self.bus.write(self.bus_address, data);
    }
}

impl<Mapper: BusDevice> BusDevice for Ppu<Mapper> {
    fn read(&mut self, address: Address) -> Option<u8> {
        Self::ADDRESS_MASK
            .remap(address)
            .map(|register| match register.0 {
                0 => self.data_latch,
                1 => self.data_latch,
                2 => self.status(),
                3 => self.data_latch,
                4 => self.read_oam(),
                5 => self.data_latch,
                6 => self.data_latch,
                7 => self.read_vram(),
                _ => unreachable!(),
            })
    }

    fn write(&mut self, address: Address, data: u8) -> bool {
        if let Some(register) = Self::ADDRESS_MASK.remap(address) {
            self.data_latch = data;

            match register {
                Address(0) => self.ctrl(data),
                Address(1) => self.mask(data),
                Address(2) => {}
                Address(3) => self.oam_address = data,
                Address(4) => self.write_oam(data),
                Address(5) => self.scroll(data),
                Address(6) => self.addr(data),
                Address(7) => self.write_vram(data),
                _ => unreachable!(),
            }

            true
        } else {
            false
        }
    }
}

struct PpuBus<Mapper: BusDevice> {
    vram_bank: RamBank<{ 2 * usize::K }>,
    mapper: Mapper,
}

impl<Mapper: BusDevice> PpuBus<Mapper> {
    pub fn new(mapper: Mapper) -> Self {
        Self {
            vram_bank: RamBank::new(AddressMask::from_block(Address(0x2000), 3, 2)),
            mapper,
        }
    }
}

impl<Mapper: BusDevice> BusDevice for PpuBus<Mapper> {
    fn read(&mut self, address: Address) -> Option<u8> {
        self.mapper
            .read(address)
            .or_else(|| self.vram_bank.read(address))
    }

    fn write(&mut self, address: Address, data: u8) -> bool {
        if !self.mapper.write(address, data) {
            self.vram_bank.write(address, data)
        } else {
            true
        }
    }
}
