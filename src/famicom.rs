use std::collections::VecDeque;

use crate::{
    isa6502::{addressing::*, *},
    *,
};

pub mod mapper;
pub mod rom;

#[derive(Debug)]
pub struct RP2A03 {
    timing: VecDeque<(fn(&mut Self), BusDirection, fn(&mut Self))>,
    pc: Address,
    stack: u8,
    bus_address: Address,
    bus_data: u8,
    a: u8,
    x: u8,
    y: u8,
    p: StatusFlags,
    opcode: u8,
    instruction: fn(&mut RP2A03),
    operand: (u8, u8),
    // result: &'_ mut u8,
    cycles: u32,
}

impl RP2A03 {
    pub fn new() -> Self {
        let mut cpu = Self {
            timing: VecDeque::with_capacity(7),
            pc: Address(0),
            stack: 0,
            bus_address: Address(0),
            bus_data: 0,
            a: 0,
            x: 0,
            y: 0,
            p: StatusFlags::Default,
            opcode: 0,
            operand: (0, 0),
            instruction: |_| unreachable!(),
            cycles: 0,
        };
        cpu.reset();
        cpu
    }

    pub fn reset(&mut self) {
        self.stack = 0;
        self.p.set(StatusFlags::Default, true);
        self.timing.clear();
        self.queue_microcode(Self::nop, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::nop, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack_push, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack_push, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack_push, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::vector::<0xFC>, BusDirection::Read, Self::set_pcl);
        self.queue_microcode(Self::vector::<0xFD>, BusDirection::Read, Self::set_pch);
        self.queue_microcode(Self::read_pc, BusDirection::Read, Self::decode_opcode);
    }

    fn decode_opcode(&mut self) {
        let opcode = self.bus_data;
        self.opcode = opcode;

        if opcode & 0x1F == 0x10 {
            self.decode_branch(opcode);
        } else {
            // let fk = ((opcode & 0xF0) >> 4, opcode, opcode & 0x3);
            match ((opcode & 0xF0) >> 4, opcode, opcode & 0x3) {
                (_, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::clc),
                (_, 0x20, _) => self.queue_jsr(),
                (_, 0x38, _) => self.decode_addressing::<Read>(opcode, Self::sec),
                (_, 0x4C, _) => self.queue_jmp(),
                (_, 0xEA, _) => self.decode_addressing::<Read>(opcode, Self::nop),
                (0xA, _, 1) => self.decode_addressing::<Read>(opcode, Self::lda),
                (0x8, _, 2) => self.decode_addressing::<Write>(opcode, Self::stx),
                (0xA, _, 2) => self.decode_addressing::<Read>(opcode, Self::ldx),

                _ => unimplemented!("No decode for {:02X}", opcode),
            }
        }
    }

    fn decode_branch(&mut self, opcode: u8) {
        let should_branch = match opcode {
            0x90 => !self.p.contains(StatusFlags::C),
            0xB0 => self.p.contains(StatusFlags::C),
            0xF0 => self.p.contains(StatusFlags::Z),
            _ => todo!("{:02X}", opcode),
        };

        self.queue_microcode(Self::read_pc, BusDirection::Read, Self::push_operand);

        if should_branch {
            self.queue_microcode(Self::nop, BusDirection::Read, |cpu| {
                cpu.pc.offset(cpu.operand.0 as i8);
                cpu.bus_address.set_low(cpu.pc.low());

                if cpu.pc != cpu.bus_address {
                    cpu.queue_microcode(Self::nop, BusDirection::Read, Self::nop);
                }
            });
        }

        self.queue_microcode(Self::read_pc, BusDirection::Read, Self::decode_opcode);
    }

    fn decode_addressing<IO: IOMode>(&mut self, opcode: u8, instruction: fn(&mut Self))
    where
        Absolute: AddressingMode<Self, IO>,
        Accumulator: AddressingMode<Self, IO>,
        Immediate: AddressingMode<Self, IO>,
        Implied: AddressingMode<Self, IO>,
        ZeroPage: AddressingMode<Self, IO>,
    {
        self.instruction = instruction;

        match opcode & 0x1f {
            0x00 | 0x02 => Immediate::enqueue(self),
            0x01 | 0x03 => unimplemented!("(d,x)"),
            0x04..=0x07 => ZeroPage::enqueue(self),
            0x08 | 0x0A => Implied::enqueue(self),
            0x09 | 0x0B => Immediate::enqueue(self),
            0x0C..=0x0F => Absolute::enqueue(self),
            0x10 | 0x12 => unimplemented!("*+d"),
            0x11 | 0x13 => unimplemented!("(d),y"),
            0x14..=0x17 => unimplemented!("d,x/y"),
            0x18 | 0x1A => Implied::enqueue(self),
            0x19 | 0x1B => unimplemented!("a,y"),
            0x1C..=0x1F => unimplemented!("a,x"),
            _ => unreachable!(),
        }
    }

    fn read_stack(&mut self) {
        self.bus_address = Address(0x100 | self.stack as u16);
    }

    fn stack_push(&mut self) {
        self.bus_address = Address(0x100 | self.stack as u16);
        self.stack = self.stack.wrapping_sub(1);
    }

    fn vector<const ABL: u8>(&mut self) {
        self.bus_address = Address(0xFF00 | ABL as u16);
    }

    fn set_pcl(&mut self) {
        self.pc.set_low(self.bus_data);
    }

    fn set_pch(&mut self) {
        self.pc.set_high(self.bus_data);
    }

    fn set_value_flags(&mut self, value: u8) {
        self.p.set(StatusFlags::Z, value == 0);
        self.p.set(StatusFlags::N, value > 0x80);
    }

    fn jmp(&mut self) {
        self.pc = Address((self.bus_data as u16) << 8 | self.operand.0 as u16);
    }

    fn lda(&mut self) {
        self.a = self.bus_data;
        self.set_value_flags(self.a);
    }

    fn ldx(&mut self) {
        self.x = self.bus_data;
        self.set_value_flags(self.x);
    }

    fn stx(&mut self) {
        self.bus_data = self.x;
    }

    fn sec(&mut self) {
        self.p.set(StatusFlags::C, true);
    }

    fn clc(&mut self) {
        self.p.set(StatusFlags::C, false);
    }
}

impl Cpu for RP2A03 {
    fn cycle(&mut self, bus: &mut impl Bus) {
        self.cycles += 1;
        match self.timing.pop_front().unwrap() {
            (pre_bus, BusDirection::Read, post_bus) => {
                pre_bus(self);
                self.bus_data = bus.read(self.bus_address);
                post_bus(self);
            }
            (pre_bus, BusDirection::Write, post_bus) => {
                pre_bus(self);
                bus.write(self.bus_address, self.bus_data);
                post_bus(self);
            }
        }
    }

    fn queue_microcode(
        &mut self,
        pre_bus: fn(&mut Self),
        bus_mode: BusDirection,
        post_bus: fn(&mut Self),
    ) {
        self.timing.push_back((pre_bus, bus_mode, post_bus));
    }

    fn decode(&mut self) {
        self.decode_opcode();
    }

    fn nop(&mut self) {}

    fn read_pc(&mut self) {
        self.bus_address = self.pc;
        self.pc.increment();
    }

    fn push_operand(&mut self) {
        self.operand.1 = self.operand.0;
        self.operand.0 = self.bus_data;
    }

    fn instruction(&mut self) {
        (self.instruction)(self)
    }

    fn zeropage(&mut self) {
        self.bus_address = Address(self.bus_data as u16);
    }

    fn queue_jsr(&mut self) {
        self.queue_microcode(Self::read_pc, BusDirection::Read, Self::push_operand);
        self.queue_microcode(Self::read_stack, BusDirection::Read, Self::nop);
        self.queue_microcode(
            |cpu| cpu.bus_data = cpu.pc.high(),
            BusDirection::Write,
            Self::stack_push,
        );
        self.queue_microcode(
            |cpu| cpu.bus_data = cpu.pc.low(),
            BusDirection::Write,
            Self::stack_push,
        );
        self.queue_microcode(Self::read_pc, BusDirection::Read, |cpu| {
            cpu.pc = Address((cpu.bus_data as u16) << 8 | cpu.operand.0 as u16)
        });

        self.queue_microcode(Self::read_pc, BusDirection::Read, Self::decode);
    }

    fn queue_jmp(&mut self) {
        self.queue_microcode(Self::read_pc, BusDirection::Read, Self::push_operand);
        self.queue_microcode(Self::read_pc, BusDirection::Read, Self::jmp);
        self.queue_microcode(Self::read_pc, BusDirection::Read, Self::decode);
    }
}

pub trait NesLogger {
    fn log(&self) -> NesTestLogEntry;
}

impl<Mapper: BusDevice> NesLogger for System<RP2A03, Mapper> {
    fn log(&self) -> NesTestLogEntry {
        NesTestLogEntry {
            pc: self.cpu.pc,
            opcode: self.cpu.opcode,
            a: self.cpu.a,
            x: self.cpu.x,
            y: self.cpu.y,
            p: self.cpu.p.bits(),
            stack: self.cpu.stack,
            cycles: self.cpu.cycles,
        }
    }
}

#[derive(Debug)]
pub struct NesTestLogEntry {
    pub pc: Address,
    pub opcode: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub stack: u8,
    pub cycles: u32,
}

impl fmt::Display for NesTestLogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{pc:?}  {op:02X}  A:{a:02X} X:{x:02X} Y:{y:02X} P:{p:02X} SP:{sp:02X}  CYC:{cycles}",
            pc = self.pc,
            op = self.opcode,
            a = self.x,
            x = self.x,
            y = self.y,
            p = self.p,
            sp = self.stack,
            cycles = self.cycles
        )
    }
}
