#![feature(test)]

use core::{fmt, num, ops, str};

use std::{
    // fmt,
    // num::ParseIntError,
    // ops::{self},
    // str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc::{self, Receiver, SendError, SyncSender},
        Arc,
    },
    thread::{self},
    time::Instant,
};

use devices::{BusDevice, RamBank};
use famicom::NesLogger;
use isa6502::*;

pub mod devices;
pub mod famicom;
pub mod isa6502;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Address(u16);

impl Address {
    pub fn new(high: u8, low: u8) -> Address {
        Address(((high as u16) << 8) | (low as u16))
    }

    fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    fn index(&self, index: u8) -> Address {
        Address::new(self.high(), self.low().wrapping_add(index))
    }

    fn offset(&mut self, offset: i8) {
        self.0 = self.0.wrapping_add_signed(offset as i16);
    }

    fn high(&self) -> u8 {
        ((self.0 & 0xff00) >> 8) as u8
    }

    fn set_high(&mut self, high: u8) {
        self.0 = (self.0 & 0xff) | ((high as u16) << 8);
    }

    fn low(&self) -> u8 {
        (self.0 & 0xff) as u8
    }

    fn set_low(&mut self, low: u8) {
        self.0 = (self.0 & 0xff00) | low as u16;
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${:04X}", self.0)
    }
}

impl ops::Add<u8> for Address {
    type Output = Address;

    fn add(self, rhs: u8) -> Self::Output {
        Address(self.0.wrapping_add(rhs as u16))
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

impl ops::Index<Address> for [u8] {
    type Output = u8;
    fn index(&self, idx: Address) -> &Self::Output {
        &self[idx.0 as usize]
    }
}

impl ops::IndexMut<Address> for [u8] {
    fn index_mut(&mut self, index: Address) -> &mut Self::Output {
        &mut self[index.0 as usize]
    }
}

impl str::FromStr for Address {
    type Err = num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Address(s.parse()?))
    }
}

pub struct AddressMask {
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

pub trait Bus {
    fn read(&self, address: Address) -> u8;
    fn write(&mut self, address: Address, data: u8);
}

pub struct System<CPU: Cpu, Mapper: BusDevice> {
    cpu: CPU,
    bus: SystemBus<Mapper>,
}

struct SystemBus<Mapper: BusDevice> {
    ram: RamBank<{ 2 * 1024 }>,
    mapper: Mapper,
}

impl<Mapper: BusDevice> SystemBus<Mapper> {
    pub fn new(mapper: Mapper) -> Self {
        Self {
            ram: RamBank::new(AddressMask::from_block(Address(0), 2, 2)),
            mapper,
        }
    }
}

impl<CPU: Cpu + Send + 'static, Mapper: BusDevice + Send + 'static> System<CPU, Mapper> {
    pub fn new(cpu: CPU, mapper: Mapper) -> Self {
        Self {
            cpu,
            bus: SystemBus::new(mapper),
        }
    }

    pub fn clock_pulse(&mut self) {
        let cpu = &mut self.cpu;
        cpu.cycle(&mut self.bus);
    }

    pub fn run(mut self, clock_signal: Receiver<u64>) {
        thread::spawn(move || {
            while let Ok(cycles) = clock_signal.recv() {
                for _ in 0..cycles {
                    self.clock_pulse();
                }
            }
        });
    }
}

impl<Mapper: BusDevice> Bus for SystemBus<Mapper> {
    fn read(&self, address: Address) -> u8 {
        self.ram
            .read(address)
            .unwrap_or_else(|| self.mapper.read(address).unwrap())
    }

    fn write(&mut self, address: Address, data: u8) {
        self.ram.write(address, data);
        self.mapper.write(address, data);
    }
}

pub struct Clock<const CLOCK_RATE: u64> {
    oscillator: SyncSender<u64>,
}

impl<const CLOCK_RATE: u64> Clock<CLOCK_RATE> {
    pub fn new() -> (Self, Receiver<u64>) {
        let one_frame = 1789733 / 60;
        let (oscillator, signal) = mpsc::sync_channel::<u64>(1);
        (Self { oscillator }, signal)
    }

    pub fn pulse(&mut self) -> Result<(), SendError<u64>> {
        self.oscillator.send(1)
    }

    pub fn run(&mut self) -> Arc<ClockControls> {
        let clock_control = Arc::new(ClockControls {
            multiplier: AtomicU8::new(0),
            divisor: AtomicU8::new(0),
            running: AtomicBool::new(true),
            cancel: AtomicBool::new(false),
        });

        let oscillator = self.oscillator.clone();
        let internal_control = clock_control.clone();

        thread::spawn(move || {
            let start = Instant::now();
            let mut cycles: u64 = 0;
            while !internal_control.cancel.load(Ordering::Relaxed) {
                let catchup_cycles =
                    ((Instant::now() - start).as_secs_f64() * CLOCK_RATE as f64) as u64 - cycles;
                if (catchup_cycles > 0) {
                    oscillator.send(catchup_cycles).unwrap();
                    cycles += catchup_cycles;
                }

                // println!("{}", catchup_cycles);
                // for _ in 0..catchup_cycles {
                //     oscillator.send(()).unwrap();
                //     cycles += 1;
                // }
                thread::yield_now();
            }
        });

        clock_control
    }
}

pub struct ClockControls {
    multiplier: AtomicU8,
    divisor: AtomicU8,
    running: AtomicBool,
    cancel: AtomicBool,
}

impl Drop for ClockControls {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

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

    impl str::FromStr for NesTestLogEntry {
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
        Address(num::ParseIntError),
        Opcode(num::ParseIntError),
        A(num::ParseIntError),
        X(num::ParseIntError),
        Y(num::ParseIntError),
        P(num::ParseIntError),
        Stack(num::ParseIntError),
        Instruction(ParseError),
        Cycles(num::ParseIntError),
    }

    #[test]
    fn decode_validation() {
        let nestest = load_nestest();

        let mut system = ntsc_system(mapper_for(nestest));

        let f = File::open("nestest.log").unwrap();
        let reader = io::BufReader::new(f);
        let lines = reader.lines();

        for line in lines {
            let line = line.unwrap();
            let expected_log = line
                .parse::<NesTestLogEntry>()
                .inspect_err(|_| eprintln!("Failed to parse nestest.log {}", line))
                .unwrap();

            let mut log = loop {
                system.clock_pulse();
                let log = system.log();
                println!("{}", log);
                // Opcode isn't fetched until the following cycle, so this is a cheap hack to correct the opcode
                if log.cycles == expected_log.cycles {
                    break log;
                }
            };

            log.opcode = system.bus.read(log.pc);
            println!("{} FIXED OPCODE", log);

            assert_eq!(
                expected_log.pc, log.pc,
                "Instruction pointer failure {:?}",
                log
            );
            assert_eq!(expected_log.opcode, log.opcode, "Opcode failure");
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
        }
    }

    fn load_nestest() -> RomImage {
        let mut nestest = RomImage::load(File::open("nestest.nes").unwrap()).unwrap();
        // Change reset vector to force automation mode for the rom
        nestest.prg_rom[0x3FFD] = 0xC0;
        nestest.prg_rom[0x3FFC] = 0x00;
        nestest
    }

    #[bench]
    fn performance_benchmark(b: &mut test::Bencher) {
        let nestest = load_nestest();

        const CYCLE_TARGET: u32 = 26554;
        b.bytes = CYCLE_TARGET as u64;
        b.iter(|| {
            let mut system = ntsc_system(mapper_for(nestest.clone()));
            for _ in 0..CYCLE_TARGET {
                system.clock_pulse();
            }
        });
    }
}
