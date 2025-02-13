use std::collections::VecDeque;

use crate::{
    isa6502::{addressing::*, *},
    *,
};

pub mod mapper;
pub mod rom;

#[derive(Debug)]
pub struct RP2A03 {
    timing: VecDeque<(fn(&mut Self) -> Address, BusDirection<Self>)>,
    pc: Address,
    stack: u8,
    address_buffer: Address,
    bus_address: Address,
    bus_data: u8,
    a: u8,
    x: u8,
    y: u8,
    p: StatusFlags,
    opcode: u8,
    // instruction: fn(&mut RP2A03, &mut u8),
    operand: (u8, u8),
    // result: &'_ mut u8,
    cycles: u32,
}

trait ReadInstructions {
    fn nop(&mut self, _: u8) {}
    fn pcl(&mut self, data: u8);
    fn pch(&mut self, data: u8);
    fn php(&mut self, _: u8) {}
}

impl ReadInstructions for RP2A03 {
    fn pcl(&mut self, data: u8) {
        self.pc.set_low(data);
    }

    fn pch(&mut self, data: u8) {
        self.pc.set_high(data);
    }
}

trait WriteInstructions {
    fn nop(&mut self, _: u8) {}
}

impl WriteInstructions for RP2A03 {}

impl RP2A03 {
    pub fn new() -> Self {
        let mut cpu = Self {
            timing: VecDeque::with_capacity(7),
            pc: Address(0),
            stack: 0,
            address_buffer: Address(0),
            bus_address: Address(0),
            bus_data: 0,
            a: 0,
            x: 0,
            y: 0,
            p: StatusFlags::Default,
            opcode: 0,
            operand: (0, 0),
            cycles: 0,
        };
        cpu.reset();
        cpu
    }

    pub fn reset(&mut self) {
        self.stack = 0;
        self.p.set(StatusFlags::Default, true);
        self.timing.clear();
        self.queue_microcode(Self::pc_inc, BusDirection::Read(ReadInstructions::nop));
        self.queue_microcode(Self::pc_inc, BusDirection::Read(ReadInstructions::nop));
        self.queue_microcode(Self::stack_push, BusDirection::Read(ReadInstructions::nop));
        self.queue_microcode(Self::stack_push, BusDirection::Read(ReadInstructions::nop));
        self.queue_microcode(Self::stack_push, BusDirection::Read(ReadInstructions::nop));
        self.queue_microcode(Self::vector::<0xFC>, BusDirection::Read(Self::pcl));
        self.queue_microcode(Self::vector::<0xFD>, BusDirection::Read(Self::pch));
        self.queue_decode();
    }

    fn decode_opcode(&mut self, opcode: &mut u8) {
        let opcode = *opcode;
        // 0000_0000
        // bit 7: high/low
        // bit 7-5: row
        // bit 4-0: column
        // bit 1-0: block
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
                (_, 0x4, 0x0, _) => self.queue_rti(),
                (_, 0x6, 0x0, _) => self.queue_rts(),
                (_, 0x2, 0x4, _) => self.decode_addressing::<Read>(opcode, Self::bit),
                (false, _, 0x8, _) => self.decode_stack(opcode),
                (_, 0x8, 0x8, _) => self.decode_addressing::<Read>(opcode, Self::dey),
                (_, 0xA, 0x8, _) => self.decode_addressing::<Read>(opcode, Self::tay),
                (_, 0xC, 0x8, _) => self.decode_addressing::<Read>(opcode, Self::iny),
                (_, 0xE, 0x8, _) => self.decode_addressing::<Read>(opcode, Self::inx),
                // (_, 0x2, 0xC, _) => self.decode_addressing::<Read>(opcode, Self::bit),
                (_, 0x4, 0xC, _) => self.queue_jmp(),
                (_, 0x0, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::clc),
                (_, 0x2, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::sec),
                (_, 0x6, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::sei),
                (_, 0xA, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::clv),
                (_, 0xC, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::cld),
                (_, 0xE, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::sed),

                (_, 0x8, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::tya),
                (_, 0x8, _, 0) => self.decode_addressing::<Write>(opcode, Self::sty),
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
                (_, 0x0, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::asl),
                (_, 0x2, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::rol),
                (_, 0x4, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::lsr),
                (_, 0x6, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::ror),
                (_, 0x8, 0xA, _) => self.decode_addressing::<Read>(opcode, Self::txa),
                (_, 0x8, 0x1A, _) => self.decode_addressing::<Read>(opcode, Self::txs),
                (_, 0x8, _, 2) => self.decode_addressing::<Write>(opcode, Self::stx),
                (_, 0xA, 0xA, _) => self.decode_addressing::<Read>(opcode, Self::tax),
                (_, 0xA, 0x1A, _) => self.decode_addressing::<Read>(opcode, Self::tsx),
                (_, 0xA, _, 2) => self.decode_addressing::<Read>(opcode, Self::ldx),
                (_, 0xC, 0xA, _) => self.decode_addressing::<Read>(opcode, Self::dex),
                // (_, 0xC, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::dec),
                (_, 0xE, 0xA, _) => self.decode_addressing::<Read>(opcode, ReadInstructions::nop),
                // (_, 0xE, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::inc),

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

        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::pull_operand));

        if should_branch {
            self.queue_microcode(
                Self::pc,
                BusDirection::Read(|cpu, _| {
                    let mut pc = cpu.pc;
                    pc.offset(cpu.operand.0 as i8);

                    if cpu.pc.high() != pc.high() {
                        cpu.push_microcode(
                            Self::pc_offset_wrapping,
                            BusDirection::Read(|cpu, _| cpu.pc.offset(cpu.operand.0 as i8)),
                        );
                    } else {
                        cpu.pc = pc;
                    }
                }),
            );
        }

        self.queue_decode();
    }

    fn decode_addressing<IO: IOMode>(&mut self, opcode: u8, instruction: fn(&mut Self, &mut u8))
    where
        Implied: AddressingMode<Self, IO>,
        IndexedIndirectX: AddressingMode<Self, IO>,
        ZeroPage: AddressingMode<Self, IO>,
        Accumulator: AddressingMode<Self, IO>,
        Immediate: AddressingMode<Self, IO>,
        Absolute: AddressingMode<Self, IO>,
        IndirectIndexedY: AddressingMode<Self, IO>,
        // ZeroPageIndexed
        // AbsoluteIndexed
    {
        self.instruction = instruction;

        match opcode & 0x1f {
            0x00 | 0x02 => Immediate::enqueue(self),
            0x01 | 0x03 => IndexedIndirectX::enqueue(self),
            0x04..=0x07 => ZeroPage::enqueue(self),
            0x08 | 0x0A => Accumulator::enqueue(self),
            0x09 | 0x0B => Immediate::enqueue(self),
            0x0C..=0x0F => Absolute::enqueue(self),
            0x10 | 0x12 => unimplemented!("*+d"),
            0x11 | 0x13 => IndirectIndexedY::enqueue(self),
            0x14..=0x17 => unimplemented!("d,x/y"),
            0x18 | 0x1A => Implied::enqueue(self),
            0x19 | 0x1B => unimplemented!("a,y"),
            0x1C..=0x1F => unimplemented!("a,x"),
            _ => unreachable!(),
        }
    }

    fn decode_stack(&mut self, opcode: u8) {
        self.queue_microcode(Self::pc, BusDirection::Read(Self::nop));

        match opcode {
            0x08 => self.queue_microcode(Self::stack_push, BusDirection::Write(Self::php)),
            0x28 => {
                self.queue_microcode(Self::stack, BusDirection::Read(Self::nop));
                self.queue_microcode(Self::stack_pull, BusDirection::Read(Self::plp));
            }
            0x48 => self.queue_microcode(Self::stack_push, BusDirection::Write(Self::pha)),
            0x68 => {
                self.queue_microcode(Self::stack, BusDirection::Read(Self::nop));
                self.queue_microcode(Self::stack_pull, BusDirection::Read(Self::pla));
            }
            _ => unreachable!(),
        }

        self.queue_decode();
    }

    fn read_stack(&mut self) {
        self.bus_address = Address(0x100 | self.stack as u16);
    }

    // fn stack_pull(&mut self) {
    //     self.bus_address = Address(0x100 | self.stack as u16);
    //     self.stack = self.stack.wrapping_add(1);
    // }

    fn push_pch(&mut self, data: &mut u8) {
        *data = self.pc.high();
    }

    fn push_pcl(&mut self, data: &mut u8) {
        *data = self.pc.low();
    }

    fn pull_pch(&mut self, data: &mut u8) {
        self.pc.set_high(*data);
    }

    fn pull_pcl(&mut self, data: &mut u8) {
        self.pc.set_low(*data);
    }

    fn pull_operand(&mut self, data: u8) {
        self.operand.0 = data;
    }

    fn pc_operand(&mut self, data: &mut u8) {
        self.pc = Address(((*data as u16) << 8) | self.operand.0 as u16);
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

    // fn nop(&mut self, _: &mut u8) {}

    fn bit(&mut self, data: &mut u8) {
        let result = self.a & *data;
        let flags = StatusFlags::N | StatusFlags::V;
        self.p.remove(flags);
        self.p
            .insert(StatusFlags::from_bits_retain((flags.bits() & *data)));
        self.p.set(StatusFlags::Z, result == 0);
    }

    fn plp(&mut self, data: &mut u8) {
        self.p.remove(StatusFlags::STACK_MASK);
        self.p
            .insert(StatusFlags::STACK_MASK.intersection(StatusFlags::from_bits_retain(*data)));
    }

    fn pha(&mut self, data: &mut u8) {
        *data = self.a;
    }

    fn pla(&mut self, data: &mut u8) {
        self.a = *data;
        self.set_value_flags(self.a);
    }

    fn dey(&mut self, _: &mut u8) {
        self.y = self.y.wrapping_sub(1);
        self.set_value_flags(self.y);
    }

    fn tay(&mut self, _: &mut u8) {
        self.y = self.a;
        self.set_value_flags(self.y);
    }

    fn iny(&mut self, _: &mut u8) {
        self.y = self.y.wrapping_add(1);
        self.set_value_flags(self.y);
    }

    fn inx(&mut self, _: &mut u8) {
        self.x = self.x.wrapping_add(1);
        self.set_value_flags(self.x);
    }

    fn jmp(&mut self, data: &mut u8) {
        self.pc = Address((*data as u16) << 8 | self.operand.0 as u16);
    }

    fn clc(&mut self, _: &mut u8) {
        self.p.set(StatusFlags::C, false);
    }

    fn sec(&mut self, _: &mut u8) {
        self.p.set(StatusFlags::C, true);
    }

    fn sei(&mut self, _: &mut u8) {
        self.p.set(StatusFlags::I, true);
    }

    fn clv(&mut self, _: &mut u8) {
        self.p.set(StatusFlags::V, false);
    }

    fn cld(&mut self, _: &mut u8) {
        self.p.set(StatusFlags::D, false);
    }

    fn sed(&mut self, _: &mut u8) {
        self.p.set(StatusFlags::D, true);
    }

    fn tya(&mut self, _: &mut u8) {
        self.a = self.y;
        self.set_value_flags(self.a);
    }

    fn sty(&mut self, data: &mut u8) {
        *data = self.y;
    }

    fn ldy(&mut self, data: &mut u8) {
        self.y = *data;
        self.set_value_flags(self.y);
    }

    fn cpy(&mut self, data: &mut u8) {
        self.p.set(StatusFlags::C, self.y >= *data);
        self.p.set(StatusFlags::Z, self.y == *data);
        self.p
            .set(StatusFlags::N, (self.y.wrapping_sub(*data) as i8) < 0);
    }

    fn cpx(&mut self, data: &mut u8) {
        self.p.set(StatusFlags::C, self.x >= *data);
        self.p.set(StatusFlags::Z, self.x == *data);
        self.p
            .set(StatusFlags::N, (self.x.wrapping_sub(*data) as i8) < 0);
    }

    fn ora(&mut self, data: &mut u8) {
        self.a = self.a | *data;
        self.set_value_flags(self.a);
    }

    fn and(&mut self, data: &mut u8) {
        self.a = self.a & *data;
        self.set_value_flags(self.a);
    }

    fn eor(&mut self, data: &mut u8) {
        self.a = self.a ^ *data;
        self.set_value_flags(self.a);
    }

    fn adc(&mut self, data: &mut u8) {
        let (result, add_overflow) = self.a.overflowing_add(*data);
        let (result, carry_overflow) = result.overflowing_add(self.p.bits() & 1);
        self.p.set(StatusFlags::C, add_overflow | carry_overflow);
        self.p.set(
            StatusFlags::V,
            (result ^ self.a) & (result ^ *data) & 0x80 > 0,
        );
        self.a = result;
        self.set_value_flags(self.a);
    }

    fn sta(&mut self, data: &mut u8) {
        *data = self.a;
    }

    fn lda(&mut self, data: &mut u8) {
        self.a = *data;
        self.set_value_flags(self.a);
    }

    fn cmp(&mut self, data: &mut u8) {
        self.p.set(StatusFlags::C, self.a >= *data);
        self.p.set(StatusFlags::Z, self.a == *data);
        self.p
            .set(StatusFlags::N, (self.a.wrapping_sub(*data) as i8) < 0);
    }

    fn sbc(&mut self, data: &mut u8) {
        let (result, add_overflow) = self.a.overflowing_add(!*data);
        let (result, carry_overflow) = result.overflowing_add(self.p.bits() & 1);
        self.p.set(StatusFlags::C, add_overflow | carry_overflow);
        self.p.set(
            StatusFlags::V,
            (result ^ self.a) & (result ^ !*data) & 0x80 > 0,
        );
        self.a = result;
        self.set_value_flags(self.a);
    }

    fn asl(&mut self, data: &mut u8) {
        self.p.set(StatusFlags::C, *data & 0b1000_0000 > 0);
        *data <<= 1;
        self.set_value_flags(*data);
    }

    fn rol(&mut self, data: &mut u8) {
        let bit0 = if self.p.contains(StatusFlags::C) {
            0b1
        } else {
            0
        };
        self.p.set(StatusFlags::C, *data & 0b1000_0000 > 0);
        *data = (*data << 1) | bit0;
        self.set_value_flags(*data);
    }

    fn lsr(&mut self, data: &mut u8) {
        self.p.set(StatusFlags::C, *data & 1 > 0);
        *data >>= 1;
        self.set_value_flags(*data);
    }

    fn ror(&mut self, data: &mut u8) {
        let bit7 = if self.p.contains(StatusFlags::C) {
            0b1000_0000
        } else {
            0
        };
        self.p.set(StatusFlags::C, *data & 1 > 0);
        *data = (*data >> 1) | bit7;
        self.set_value_flags(*data);
    }

    fn txa(&mut self, data: &mut u8) {
        *data = self.x;
        self.set_value_flags(*data);
    }

    fn txs(&mut self, _: &mut u8) {
        self.stack = self.x;
    }

    fn stx(&mut self, data: &mut u8) {
        *data = self.x;
    }

    fn tax(&mut self, data: &mut u8) {
        self.x = *data;
        self.set_value_flags(self.x);
    }

    fn tsx(&mut self, _: &mut u8) {
        self.x = self.stack;
        self.set_value_flags(self.x);
    }

    fn ldx(&mut self, data: &mut u8) {
        self.x = *data;
        self.set_value_flags(self.x);
    }

    fn dex(&mut self, data: &mut u8) {
        self.x = self.x.wrapping_sub(1);
        self.set_value_flags(self.x);
    }

    fn dec(&mut self) {
        self.bus_data = self.bus_data.wrapping_sub(1);
        self.set_value_flags(self.bus_data);
    }

    fn inc(&mut self) {
        self.bus_data = self.bus_data.wrapping_add(1);
        self.set_value_flags(self.bus_data);
    }
}

impl AddressMode for RP2A03 {
    fn address(&mut self) -> Address {
        self.address_buffer
    }

    fn address_indexedx(&mut self) -> Address {
        self.address_buffer.index(self.x)
    }

    fn address_inc(&mut self) -> Address {
        self.address_buffer.index(1)
    }

    fn buffer(&mut self, address: Address) -> Address {
        self.address_buffer = address;
        address
    }

    fn pc(&mut self) -> Address {
        self.pc
    }

    fn pc_inc(&mut self) -> Address {
        let address = self.pc;
        self.pc.increment();
        address
    }

    fn pc_offset_wrapping(&mut self) -> Address {
        Address(
            ((self.pc.high() as u16) << 8)
                | self.pc.low().wrapping_add_signed(self.operand.0 as i8) as u16,
        )
    }

    fn stack(&mut self) -> Address {
        let address = Address(0x100 | self.stack as u16);
        address
    }

    fn stack_pull(&mut self) -> Address {
        self.stack = self.stack.wrapping_add(1);
        Address(0x100 | self.stack as u16)
    }

    fn stack_push(&mut self) -> Address {
        let address = Address(0x100 | self.stack as u16);
        self.stack = self.stack.wrapping_sub(1);
        address
    }

    fn vector<const VECTOR: u8>(&mut self) -> Address {
        Address(0xFF00 | VECTOR as u16)
    }

    fn zeropage(&mut self) -> Address {
        println!("ZEROPAGE");
        Address(self.operand.0 as u16)
    }
}

impl Cpu for RP2A03 {
    fn cycle(&mut self, bus: &mut impl Bus) {
        self.cycles += 1;
        match self.timing.pop_front().unwrap() {
            (address_mode, BusDirection::Read(operation)) => {
                self.bus_data = bus.read(address_mode(self));
                operation(self, self.bus_data);
            }
            (address_mode, BusDirection::Write(operation)) => {
                self.bus_data = operation(self);
                bus.write(address_mode(self), self.bus_data);
            }
        }
    }

    fn push_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection<Self>,
    ) {
        self.timing.push_front((address_mode, bus_mode, operation));
    }

    fn queue_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection<Self>,
    ) {
        self.timing.push_back((address_mode, bus_mode, operation));
    }

    fn queue_decode(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::decode_opcode);
    }

    // fn decode(&mut self) {
    //     self.decode_opcode();
    // }

    fn read_pc(&mut self) {
        self.bus_address = self.pc;
    }

    fn read_pc_inc(&mut self) {
        self.bus_address = self.pc;
        self.pc.increment();
    }

    fn pull_operand(&mut self, data: &mut u8) {
        self.operand.1 = self.operand.0;
        self.operand.0 = *data;
    }

    fn address_operand(&mut self, data: &mut u8) {
        self.address_buffer = Address((*data as u16) << 8 | self.operand.0 as u16);
    }

    fn instruction(&mut self, data: &mut u8) {
        (self.instruction)(self, data)
    }

    fn with_accumulator(&mut self, operation: fn(&mut Self, data: &mut u8)) {
        let mut data = self.a;
        operation(self, &mut data);
        self.a = data;
    }

    // fn load_accumulator(&mut self) {
    //     self.a = self.bus_data;
    // }

    // fn store_accumulator(&mut self) {
    //     self.bus_data = self.a;
    // }

    fn queue_jmp(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::pull_operand);
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::jmp);
        self.queue_decode();
    }

    fn queue_jsr(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::pull_operand);
        self.queue_microcode(Self::stack, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack_push, BusDirection::Write, Self::push_pch);
        self.queue_microcode(Self::stack_push, BusDirection::Write, Self::push_pcl);
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::pc_operand);
        self.queue_decode();
    }

    fn queue_rti(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::plp);
        self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::pull_pcl);
        self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::pull_pch);
        self.queue_decode();
    }

    fn queue_rts(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::pull_pcl);
        self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::pull_pch);
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::nop);
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
