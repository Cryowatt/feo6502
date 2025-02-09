use std::{fmt, num::ParseIntError, ops, str::FromStr};

use isa6502::Cpu;

pub mod famicom;
pub mod isa6502;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Address(u16);

impl Address {
    fn increment(&mut self) {
        self.0 += 1;
    }

    fn set_high(&mut self, high: u8) {
        self.0 = (self.0 & 0xff) | (high as u16) << 8;
    }

    fn set_low(&mut self, low: u8) {
        self.0 = (self.0 & 0xff00) | low as u16;
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${:04X}", self.0)
        // f.pad_integral(true, "0"., prefix, buf)
        // fmt::UpperHex::fmt(&self.0, f)
        // f.debug_tuple("Address").field(&self.0)..finish()
    }
}

impl ops::AddAssign<u16> for Address {
    fn add_assign(&mut self, rhs: u16) {
        self.0 += rhs;
    }
}

impl ops::BitAnd<u16> for Address {
    type Output = Address;

    fn bitand(self, rhs: u16) -> Self::Output {
        Address(self.0 & rhs)
    }
}

impl ops::BitAnd<Address> for u16 {
    type Output = Address;

    fn bitand(self, rhs: Address) -> Self::Output {
        Address(self & rhs.0)
    }
}

impl ops::Index<Address> for Vec<u8> {
    type Output = u8;
    fn index(&self, idx: Address) -> &Self::Output {
        &self[idx.0 as usize]
    }
}

impl FromStr for Address {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Address(s.parse()?))
    }
}

struct AddressMask {
    start_address: Address,
    address_mask: u16,
    mirror_mask: u16,
}

impl AddressMask {
    fn from_block(start_address: Address, prefix_bits: u8, mirror_bits: u8) -> Self {
        Self {
            start_address,
            address_mask: !(0xffff >> prefix_bits),
            mirror_mask: (0xffff >> (prefix_bits + mirror_bits)),
        }
    }

    fn remap(&self, address: Address) -> Option<Address> {
        if self.address_mask & address == self.start_address {
            Some(address & self.mirror_mask)
        } else {
            None
        }
    }
}

enum Operands {
    None,
    Operand(u8),
    Address(Address),
}

mod mos6502 {
    use strum_macros::EnumString;

    #[derive(Debug, PartialEq, EnumString)]
    pub enum Instruction {
        BRK,
        JMP,
        #[strum(props(Mnemomic = "JMP"))]
        JIA,
        LDA,
        LDX,
        LDY,
        STA,
        STX,
        STY,
        Illegal,
    }

    pub enum OpCode {
        Absolute(Instruction),
        AbsoluteIndexedIndrectX(Instruction),
        AbsoluteIndexedX(Instruction),
        AbsoluteIndexedY(Instruction),
        AbsoluteIndirect(Instruction),
        Accumulator(Instruction),
        Immediate(Instruction),
        Implied(Instruction),
        Jam,
        ProgramCounterRelative(Instruction),
        Stack(Instruction),
        ZeroPage(Instruction),
        ZeroPageIndexedIndirectX(Instruction),
        ZeroPageIndexedX(Instruction),
        ZeroPageIndexedY(Instruction),
        ZeroPageIndirect(Instruction),
        ZeroPageIndirectIndexedY(Instruction),
    }

    impl OpCode {
        pub fn as_instruction(self) -> Instruction {
            match self {
                OpCode::Absolute(instruction) => instruction,
                OpCode::AbsoluteIndexedIndrectX(instruction) => instruction,
                OpCode::AbsoluteIndexedX(instruction) => instruction,
                OpCode::AbsoluteIndexedY(instruction) => instruction,
                OpCode::AbsoluteIndirect(instruction) => instruction,
                OpCode::Accumulator(instruction) => instruction,
                OpCode::Immediate(instruction) => instruction,
                OpCode::Implied(instruction) => instruction,
                OpCode::ProgramCounterRelative(instruction) => instruction,
                OpCode::Stack(instruction) => instruction,
                OpCode::ZeroPage(instruction) => instruction,
                OpCode::ZeroPageIndexedIndirectX(instruction) => instruction,
                OpCode::ZeroPageIndexedX(instruction) => instruction,
                OpCode::ZeroPageIndexedY(instruction) => instruction,
                OpCode::ZeroPageIndirect(instruction) => instruction,
                OpCode::ZeroPageIndirectIndexedY(instruction) => instruction,
                _ => Instruction::Illegal,
            }
        }
    }
}

#[derive(Debug)]
pub enum BusMode {
    Write,
    Read,
}

pub trait Bus {
    fn read(&self, address: Address) -> u8;
    fn write(&mut self, address: Address, data: u8);
}

struct System<CPU: Cpu, Mapper: BusDevice> {
    cpu: CPU,
    cpu_divisor: u8,
    bus: SystemBus<Mapper>,
}

struct SystemBus<Mapper: BusDevice> {
    mapper: Mapper,
}

impl<CPU: Cpu, Mapper: BusDevice> System<CPU, Mapper> {
    pub fn new(cpu: CPU, cpu_divisor: u8, mapper: Mapper) -> Self {
        Self {
            cpu,
            cpu_divisor,
            bus: SystemBus { mapper },
        }
    }

    pub fn clock_pulse(&mut self) {
        let cpu = &mut self.cpu;
        cpu.cycle(&mut self.bus);
    }
}

impl<Mapper: BusDevice> Bus for SystemBus<Mapper> {
    fn read(&self, address: Address) -> u8 {
        self.mapper.read(address).unwrap_or_default()
    }

    fn write(&mut self, address: Address, data: u8) {
        self.mapper.write(address, data);
    }
}

pub trait BusDevice {
    fn read(&self, address: Address) -> Option<u8>;
    fn write(&mut self, address: Address, data: u8);
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{self, BufRead},
    };

    use strum::ParseError;

    use crate::famicom::{
        mapper::mapper_for,
        rom::{ntsc_system, RomImage},
        *,
    };

    use super::*;

    impl FromStr for NesTestLogEntry {
        type Err = NesTestLogEntryParseError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let pc = Address(
                u16::from_str_radix(&s[0..4], 16)
                    .map_err(|e| NesTestLogEntryParseError::Address(e))?,
            );
            let opcode = u8::from_str_radix(&s[6..8], 16)
                .map_err(|e| NesTestLogEntryParseError::Opcode(e))?;
            let cycles = s[90..]
                .parse()
                .map_err(|e| NesTestLogEntryParseError::Cycles(e))?;
            let a =
                u8::from_str_radix(&s[50..52], 16).map_err(|e| NesTestLogEntryParseError::A(e))?;
            let x =
                u8::from_str_radix(&s[55..57], 16).map_err(|e| NesTestLogEntryParseError::X(e))?;
            let y =
                u8::from_str_radix(&s[60..62], 16).map_err(|e| NesTestLogEntryParseError::Y(e))?;
            let p =
                u8::from_str_radix(&s[65..67], 16).map_err(|e| NesTestLogEntryParseError::P(e))?;
            let stack = u8::from_str_radix(&s[71..73], 16)
                .map_err(|e| NesTestLogEntryParseError::Stack(e))?;

            Ok(Self {
                pc,
                opcode,
                a,
                x,
                y,
                p,
                stack,
                // instruction,
                cycles,
            })
        }
    }

    #[derive(Debug)]
    pub enum NesTestLogEntryParseError {
        Address(ParseIntError),
        Opcode(ParseIntError),
        A(ParseIntError),
        X(ParseIntError),
        Y(ParseIntError),
        P(ParseIntError),
        Stack(ParseIntError),
        Instruction(ParseError),
        Cycles(ParseIntError),
    }

    #[test]
    fn decode_validation() {
        let mut nestest = RomImage::load(File::open("nestest.nes").unwrap()).unwrap();
        // Change reset vector to force automation mode for the rom
        nestest.prg_rom[0x3FFD] = 0xC0;
        nestest.prg_rom[0x3FFC] = 0x00;

        let mut system = ntsc_system(mapper_for(nestest));
        // let fk = (system.mapper.read(Address(0xfffc)).unwrap() as u16)
        //     | (system.mapper.read(Address(0xfffd)).unwrap() as u16) << 8;
        // println!("Fk {:04X}", fk);
        // system.clock_pulse();

        let f = File::open("nestest.log").unwrap();
        let reader = io::BufReader::new(f);
        let lines = reader.lines();
        let mut current_opcode = 0;

        for line in lines {
            let line = line.unwrap();
            let expected_log = line
                .parse::<NesTestLogEntry>()
                .inspect_err(|_| eprintln!("Failed to parse nestest.log {}", line))
                .unwrap();

            let log = loop {
                system.clock_pulse();
                let log = system.log();

                if log.cycles == expected_log.cycles {
                    println!("{}", log);
                    break log;
                }
            };

            assert_eq!(
                expected_log.pc, log.pc,
                "Instruction pointer failure {:?}",
                log
            );
            // assert_eq!(expected_log.opcode, log.opcode, "Opcode failure");
            assert_eq!(log.a, expected_log.a, "A register failure {:?}", log);
            assert_eq!(log.x, expected_log.x, "X register failure {:?}", log);
            assert_eq!(log.y, expected_log.y, "Y register failure {:?}", log);
            assert_eq!(
                log.p, expected_log.p,
                "Status register failure {:08b} should be {:08b} {:?}",
                log.p, expected_log.p, log
            );
            assert_eq!(
                expected_log.stack, log.stack,
                "Stack pointer failure {:?}",
                log
            );

            // let decoded = famicom::RP2A03::decode(entry.opcode)
            //     .inspect_err(|err| eprintln!("Failed to decode {:02X}", err))
            //     .unwrap();
            // eprintln!("{:?}", system.cycles);
            // eprintln!("{:?}", expected_log);
            // assert!(false, "FUCK");
            // break;
            // assert_eq!(
            //     decoded.as_instruction(),
            //     entry.instruction,
            //     "Decode mismatch {:02X}",
            //     entry.opcode
            // );
        }
    }
}
// Decode test:
// Read address from nestest.log, match operands and decode
