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
        self.queue_decode();
    }

    fn decode_opcode(&mut self) {
        // 0000_0000
        // bit 7: high/low
        // bit 7-5: row
        // bit 4-0: column
        // bit 1-0: block
        let opcode = self.bus_data;
        let high = (opcode & 0b1000_0000) > 0;
        let row = (opcode & 0b1110_0000) >> 4;
        let column = opcode & 0b0001_1111;
        let block = opcode & 0b0000_0011;
        self.opcode = opcode;

        // println!("{} {:X} {:X} {}", high, row, column, block);
        if opcode & 0x1F == 0x10 {
            self.decode_branch(opcode);
        } else {
            match (high, row, column, block) {
                // Control
                (_, 0x2, 0x0, _) => self.queue_jsr(),
                (_, 0x6, 0x0, _) => self.queue_rts(),
                (_, 0x2, 0x4, _) => self.decode_addressing::<Read>(opcode, Self::bit),
                (false, _, 0x8, _) => self.decode_stack(opcode),
                (_, 0x4, 0xC, _) => self.queue_jmp(),
                (true, _, 0x8, _) => self.decode_stack(opcode),
                (_, 0x0, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::clc),
                (_, 0x2, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::sec),
                (_, 0x6, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::sei),
                (_, 0xA, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::clv),
                (_, 0xC, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::cld),
                (_, 0xE, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::sed),

                (_, 0xA, _, 0) => self.decode_addressing::<Read>(opcode, Self::ldy),
                (_, 0xC, _, 0) => self.decode_addressing::<Read>(opcode, Self::cpy),
                (_, 0xE, _, 0) => self.decode_addressing::<Read>(opcode, Self::cpx),

                // ALU
                (_, 0x0, _, 1) => self.decode_addressing::<Read>(opcode, Self::ora),
                (_, 0x2, _, 1) => self.decode_addressing::<Read>(opcode, Self::and),
                (_, 0x4, _, 1) => self.decode_addressing::<Read>(opcode, Self::eor),
                (_, 0x6, _, 1) => self.decode_addressing::<Read>(opcode, Self::adc),
                (_, 0x8, _, 1) => self.decode_addressing::<Write>(opcode, Self::sta),
                (_, 0xA, _, 1) => self.decode_addressing::<Read>(opcode, Self::lda),
                (_, 0xC, _, 1) => self.decode_addressing::<Read>(opcode, Self::cmp),
                (_, 0xE, _, 1) => self.decode_addressing::<Read>(opcode, Self::sbc),
                // RMW
                (_, 0x8, _, 2) => self.decode_addressing::<Write>(opcode, Self::stx),
                (_, 0xA, _, 2) => self.decode_addressing::<Read>(opcode, Self::ldx),
                (_, 0xE, 0xA, _) => self.decode_addressing::<Read>(opcode, Self::nop),
                // Illegal
                _ => unimplemented!("No decode for {:02X}", opcode),
            }
        }
    }

    fn decode_branch(&mut self, opcode: u8) {
        let should_branch = match opcode {
            0x10 => !self.p.contains(StatusFlags::N),
            0x30 => self.p.contains(StatusFlags::N),
            0x50 => !self.p.contains(StatusFlags::V),
            0x70 => self.p.contains(StatusFlags::V),
            0x90 => !self.p.contains(StatusFlags::C),
            0xB0 => self.p.contains(StatusFlags::C),
            0xD0 => !self.p.contains(StatusFlags::Z),
            0xF0 => self.p.contains(StatusFlags::Z),
            _ => todo!("{:02X}", opcode),
        };

        self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::push_operand);

        if should_branch {
            self.queue_microcode(Self::nop, BusDirection::Read, |cpu| {
                cpu.pc.offset(cpu.operand.0 as i8);
                cpu.bus_address.set_low(cpu.pc.low());

                if cpu.pc != cpu.bus_address {
                    cpu.queue_microcode(Self::nop, BusDirection::Read, Self::nop);
                }
            });
        }

        self.queue_decode();
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

    fn decode_stack(&mut self, opcode: u8) {
        // push
        // 1    PC     R  fetch opcode, increment PC
        // 2    PC     R  read next instruction byte (and throw it away)
        // 3  $0100,S  W  push register on stack, decrement S

        self.queue_microcode(Self::read_pc, BusDirection::Read, Self::nop);

        match opcode {
            0x08 => self.queue_microcode(Self::php, BusDirection::Write, Self::nop),
            0x28 => {
                self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::nop);
                self.queue_microcode(Self::read_stack, BusDirection::Read, Self::plp);
            }
            0x48 => self.queue_microcode(Self::pha, BusDirection::Write, Self::nop),
            0x68 => {
                self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::nop);
                self.queue_microcode(Self::read_stack, BusDirection::Read, Self::pla);
            }
            _ => unreachable!(),
        }

        self.queue_decode();
    }

    fn read_stack(&mut self) {
        self.bus_address = Address(0x100 | self.stack as u16);
    }

    fn stack_push(&mut self) {
        self.bus_address = Address(0x100 | self.stack as u16);
        self.stack = self.stack.wrapping_sub(1);
    }

    fn stack_pull(&mut self) {
        self.bus_address = Address(0x100 | self.stack as u16);
        self.stack = self.stack.wrapping_add(1);
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
        self.p.set(StatusFlags::N, (value as i8) < 0);
    }

    fn bit(&mut self) {
        let result = self.a & self.bus_data;
        let flags = StatusFlags::N | StatusFlags::V;
        self.p.remove(flags);
        self.p.insert(StatusFlags::from_bits_retain(
            (flags.bits() & self.bus_data),
        ));
        self.p.set(StatusFlags::Z, result == 0);
    }

    fn jmp(&mut self) {
        self.pc = Address((self.bus_data as u16) << 8 | self.operand.0 as u16);
    }

    fn php(&mut self) {
        self.stack_push();
        self.bus_data = self.p.union(StatusFlags::STACK_MASK.complement()).bits();
    }

    fn plp(&mut self) {
        self.p.remove(StatusFlags::STACK_MASK);
        self.p.insert(
            StatusFlags::STACK_MASK.intersection(StatusFlags::from_bits_retain(self.bus_data)),
        );
    }

    fn pha(&mut self) {
        self.stack_push();
        self.bus_data = self.a;
    }

    fn pla(&mut self) {
        self.a = self.bus_data;
        self.set_value_flags(self.a);
    }

    fn clc(&mut self) {
        self.p.set(StatusFlags::C, false);
    }

    fn sec(&mut self) {
        self.p.set(StatusFlags::C, true);
    }

    fn sei(&mut self) {
        self.p.set(StatusFlags::I, true);
    }

    fn clv(&mut self) {
        self.p.set(StatusFlags::V, false);
    }

    fn cld(&mut self) {
        self.p.set(StatusFlags::D, false);
    }

    fn sed(&mut self) {
        self.p.set(StatusFlags::D, true);
    }

    fn ldy(&mut self) {
        self.y = self.bus_data;
        self.set_value_flags(self.y);
    }

    fn cpy(&mut self) {
        self.p.set(StatusFlags::C, self.y >= self.bus_data);
        self.p.set(StatusFlags::Z, self.y == self.bus_data);
        self.p.set(
            StatusFlags::N,
            (self.y.wrapping_sub(self.bus_data) as i8) < 0,
        );
    }

    fn cpx(&mut self) {
        self.p.set(StatusFlags::C, self.x >= self.bus_data);
        self.p.set(StatusFlags::Z, self.x == self.bus_data);
        self.p.set(
            StatusFlags::N,
            (self.x.wrapping_sub(self.bus_data) as i8) < 0,
        );
    }

    fn ora(&mut self) {
        self.a = self.a | self.bus_data;
        self.set_value_flags(self.a);
    }

    fn and(&mut self) {
        self.a = self.a & self.bus_data;
        self.set_value_flags(self.a);
    }

    fn eor(&mut self) {
        self.a = self.a ^ self.bus_data;
        self.set_value_flags(self.a);
    }

    fn adc(&mut self) {
        let (result, add_overflow) = self.a.overflowing_add(self.bus_data);
        let (result, carry_overflow) = result.overflowing_add(self.p.bits() & 1);
        self.p.set(StatusFlags::C, add_overflow | carry_overflow);
        self.p.set(
            StatusFlags::V,
            (result ^ self.a) & (result ^ self.bus_data) & 0x80 > 0,
        );
        self.a = result;
        self.set_value_flags(self.a);
    }

    fn sta(&mut self) {
        self.bus_data = self.a;
    }

    fn lda(&mut self) {
        self.a = self.bus_data;
        self.set_value_flags(self.a);
    }

    fn cmp(&mut self) {
        self.p.set(StatusFlags::C, self.a >= self.bus_data);
        self.p.set(StatusFlags::Z, self.a == self.bus_data);
        self.p.set(
            StatusFlags::N,
            (self.a.wrapping_sub(self.bus_data) as i8) < 0,
        );
    }

    fn sbc(&mut self) {
        let (result, add_overflow) = self.a.overflowing_add(!self.bus_data);
        let (result, carry_overflow) = result.overflowing_add(self.p.bits() & 1);
        self.p.set(StatusFlags::C, add_overflow | carry_overflow);
        self.p.set(
            StatusFlags::V,
            (result ^ self.a) & (result ^ !self.bus_data) & 0x80 > 0,
        );
        self.a = result;
        self.set_value_flags(self.a);
    }

    fn stx(&mut self) {
        self.bus_data = self.x;
    }

    fn ldx(&mut self) {
        self.x = self.bus_data;
        self.set_value_flags(self.x);
    }
}

impl Cpu for RP2A03 {
    fn cycle(&mut self, bus: &mut impl Bus) {
        self.cycles += 1;
        match self.timing.pop_front().unwrap() {
            (pre_bus, BusDirection::Read, post_bus) => {
                pre_bus(self);
                self.bus_data = bus.read(self.bus_address);
                eprintln!("{:?} => {:02X}", self.bus_address, self.bus_data);
                post_bus(self);
            }
            (pre_bus, BusDirection::Write, post_bus) => {
                pre_bus(self);
                bus.write(self.bus_address, self.bus_data);
                eprintln!("{:?} <= {:02X}", self.bus_address, self.bus_data);
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

    fn queue_decode(&mut self) {
        self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::decode_opcode);
    }

    fn decode(&mut self) {
        self.decode_opcode();
    }

    fn nop(&mut self) {}

    fn read_pc(&mut self) {
        self.bus_address = self.pc;
    }

    fn read_pc_inc(&mut self) {
        self.bus_address = self.pc;
        self.pc.increment();
    }

    fn push_operand(&mut self) {
        self.operand.1 = self.operand.0;
        self.operand.0 = self.bus_data;
    }

    fn address_operand(&mut self) {
        self.bus_address = Address((self.operand.0 as u16) << 8 | self.operand.1 as u16);
    }

    fn instruction(&mut self) {
        (self.instruction)(self)
    }

    fn zeropage(&mut self) {
        self.bus_address = Address(self.bus_data as u16);
    }

    fn queue_jmp(&mut self) {
        self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::push_operand);
        self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::jmp);
        self.queue_decode();
    }

    fn queue_jsr(&mut self) {
        self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::push_operand);
        self.queue_microcode(Self::read_stack, BusDirection::Read, Self::stack_push);
        self.queue_microcode(
            |cpu| cpu.bus_data = cpu.pc.high(),
            BusDirection::Write,
            Self::stack_push,
        );
        self.queue_microcode(
            |cpu| cpu.bus_data = cpu.pc.low(),
            BusDirection::Write,
            Self::nop,
        );
        self.queue_microcode(Self::read_pc_inc, BusDirection::Read, |cpu| {
            cpu.pc = Address((cpu.bus_data as u16) << 8 | cpu.operand.0 as u16)
        });

        self.queue_decode();
    }

    fn queue_rts(&mut self) {
        self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::set_pcl);
        self.queue_microcode(Self::read_stack, BusDirection::Read, Self::set_pch);
        self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::nop);
        self.queue_decode();
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
            a = self.a,
            x = self.x,
            y = self.y,
            p = self.p,
            sp = self.stack,
            cycles = self.cycles
        )
    }
}
