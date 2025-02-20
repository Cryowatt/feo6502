use std::io;

use bitfields::bitfield;
use byteorder::{BigEndian, ByteOrder as _, ReadBytesExt};
use strum_macros::FromRepr;

use crate::{BusDevice, System};

use super::RP2A03;

macro_rules! from_bits {
    ( $enum:ident, $repr:ty ) => {
        impl $enum {
            const fn from_bits(bits: $repr) -> Self {
                Self::from_repr(bits).expect("Enum value should be valid")
            }

            const fn into_bits(self) -> $repr {
                self as $repr
            }
        }
    };
}

#[repr(u8)]
#[derive(FromRepr, Clone, Copy)]
pub enum NametableLayout {
    Vertical = 0,
    Horizontal = 1,
}
from_bits!(NametableLayout, u8);

#[bitfield(u8)]
struct Flags6 {
    #[bits(1)]
    nametable_layout: NametableLayout,

    #[bits(1)]
    has_nonvolatile_memory: bool,

    #[bits(1)]
    has_trainer_header: bool,

    #[bits(1)]
    enable_alternative_nametables: bool,

    #[bits(4)]
    mapper_low_nibble: u8,
}

#[repr(u8)]
#[derive(FromRepr)]
enum ConsoleType {
    Famicom = 0,
    VsSystem = 1,
    Playchoice10 = 2,
    Extended = 3,
}
from_bits!(ConsoleType, u8);

#[repr(u8)]
#[derive(FromRepr)]
enum INesFormat {
    INes = 0,
    Nes2_0 = 2,
}
from_bits!(INesFormat, u8);

#[bitfield(u8)]
struct Flags7 {
    #[bits(2)]
    console_type: ConsoleType,

    #[bits(2)]
    format_version: INesFormat,

    #[bits(4)]
    mapper_mid_nibble: u8,
}

#[derive(Clone)]
pub struct RomImage {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub prg_ram_size: usize,
    pub mapper: u16,
    pub submapper: u8,
    pub nametable_layout: NametableLayout,
}

impl RomImage {
    pub fn load<R: io::Read + io::Seek>(mut reader: R) -> Result<Self, io::Error> {
        let ines_header: u32 = byteorder::BigEndian::read_u32(b"NES\x1a");
        // let fk = b"1";
        let header = reader.read_u32::<BigEndian>()?;

        if header != ines_header {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown format"));
        }
        let prg_rom_size = reader.read_u8()?;
        let chr_rom_size = reader.read_u8()?;

        let flags6 = Flags6::from_bits(reader.read_u8()?);
        let flags7 = Flags7::from_bits(reader.read_u8()?);

        match flags7.format_version() {
            INesFormat::INes => {
                Self::load_ines_image(prg_rom_size, chr_rom_size, flags6, flags7, reader)
            }
            INesFormat::Nes2_0 => {
                Self::load_nes2_image(prg_rom_size, chr_rom_size, flags6, flags7, reader)
            }
        }
    }

    fn load_ines_image<R: io::Read + io::Seek>(
        prg_rom_size: u8,
        chr_rom_size: u8,
        flags6: Flags6,
        flags7: Flags7,
        mut reader: R,
    ) -> Result<Self, io::Error> {
        if reader.stream_position().unwrap() != 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Image deserialization failure",
            ));
        }

        let prg_ram_size = (reader.read_u8()? as usize) * 0x2000;
        // PRG ROM size is defined as number of 16KB units.
        let prg_rom_size = (prg_rom_size as usize) * 0x4000;
        // CHR ROM size is defined as number of 8KB units.
        let chr_rom_size = (chr_rom_size as usize) * 0x2000;

        reader.seek(io::SeekFrom::Start(16))?;

        if flags6.has_trainer_header() {
            unimplemented!();
        }

        let mut prg_rom = vec![0; prg_rom_size];
        reader.read_exact(prg_rom.as_mut_slice())?;
        let mut chr_rom = vec![0; chr_rom_size];
        reader.read_exact(chr_rom.as_mut_slice())?;

        let mapper: u16 =
            ((flags7.mapper_mid_nibble() as u16) << 4) | (flags6.mapper_low_nibble() as u16);

        Ok(Self {
            prg_rom,
            chr_rom,
            prg_ram_size,
            mapper,
            submapper: 0,
            nametable_layout: flags6.nametable_layout(),
        })
    }

    fn load_nes2_image<R: io::Read + io::Seek>(
        _prg_rom_size: u8,
        _chr_rom_size: u8,
        flags6: Flags6,
        flags7: Flags7,
        mut reader: R,
    ) -> Result<Self, io::Error> {
        if reader.stream_position().unwrap() != 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Image deserialization failure",
            ));
        }

        let mapper_msb = reader.read_u8()?;
        let _mapper: u16 = ((mapper_msb as u16 & 0xf) << 8)
            | ((flags7.mapper_mid_nibble() as u16) << 4)
            | (flags6.mapper_low_nibble() as u16);
        let _submapper = mapper_msb >> 4;
        let _rom_size_msb = reader.read_u8()?;

        todo!()
    }
}

pub fn ntsc_system<Mapper: BusDevice + Send + 'static>(mapper: Mapper) -> System<RP2A03, Mapper> {
    System::new(RP2A03::new(), mapper)
}
