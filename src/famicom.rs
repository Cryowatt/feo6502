use std::collections::VecDeque;

use crate::{
    isa6502::{addressing::*, *},
    *,
};

pub mod mapper;
pub mod rom;

#[derive(Debug)]
pub struct RP2A03 {
    timing: VecDeque<(
        fn(&mut Self) -> Address,
        BusDirection,
        fn(&mut Self, &mut u8),
    )>,
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
    instruction: fn(&mut RP2A03, &mut u8),
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
            address_buffer: Address(0),
            bus_address: Address(0),
            bus_data: 0,
            a: 0,
            x: 0,
            y: 0,
            p: StatusFlags::Default,
            opcode: 0,
            operand: (0, 0),
            instruction: |_, _| unreachable!(),
            cycles: 0,
        };
        cpu.reset();
        cpu
    }

    pub fn reset(&mut self) {
        self.stack = 0;
        self.p.set(StatusFlags::Default, true);
        self.timing.clear();
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::nop);
        self.queue_microcode(Self::stack_push, BusDirection::Read, Self::push_pch);
        self.queue_microcode(Self::stack_push, BusDirection::Read, Self::push_pcl);
        self.queue_microcode(Self::stack_push, BusDirection::Read, Self::push_p);
        self.queue_microcode(Self::vector::<0xFC>, BusDirection::Read, Self::pull_pcl);
        self.queue_microcode(Self::vector::<0xFD>, BusDirection::Read, Self::pull_pch);
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
                // (_, 0x4, 0x0, _) => self.queue_rti(),
                // (_, 0x6, 0x0, _) => self.queue_rts(),
                // (_, 0x2, 0x4, _) => self.decode_addressing::<Read>(opcode, Self::bit),
                // (false, _, 0x8, _) => self.decode_stack(opcode),
                // (_, 0x8, 0x8, _) => self.decode_addressing::<Read>(opcode, Self::dey),
                // (_, 0xA, 0x8, _) => self.decode_addressing::<Read>(opcode, Self::tay),
                // (_, 0xC, 0x8, _) => self.decode_addressing::<Read>(opcode, Self::iny),
                // (_, 0xE, 0x8, _) => self.decode_addressing::<Read>(opcode, Self::inx),
                // (_, 0x2, 0xC, _) => self.decode_addressing::<Read>(opcode, Self::bit),
                (_, 0x4, 0xC, _) => self.queue_jmp(),
                // (_, 0x0, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::clc),
                (_, 0x2, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::sec),
                // (_, 0x6, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::sei),
                // (_, 0xA, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::clv),
                // (_, 0xC, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::cld),
                // (_, 0xE, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::sed),

                // (_, 0x8, 0x18, _) => self.decode_addressing::<Read>(opcode, Self::tya),
                // (_, 0x8, _, 0) => self.decode_addressing::<Write>(opcode, Self::sty),
                // (_, 0xA, _, 0) => self.decode_addressing::<Read>(opcode, Self::ldy),
                // (_, 0xC, _, 0) => self.decode_addressing::<Read>(opcode, Self::cpy),
                // (_, 0xE, _, 0) => self.decode_addressing::<Read>(opcode, Self::cpx),

                // ALU
                // (_, 0x0, _, 1) => self.decode_addressing::<Read>(opcode, Self::ora),
                // (_, 0x2, _, 1) => self.decode_addressing::<Read>(opcode, Self::and),
                // (_, 0x4, _, 1) => self.decode_addressing::<Read>(opcode, Self::eor),
                // (_, 0x6, _, 1) => self.decode_addressing::<Read>(opcode, Self::adc),
                // (_, 0x8, _, 1) => self.decode_addressing::<Write>(opcode, Self::sta),
                // (_, 0xA, _, 1) => self.decode_addressing::<Read>(opcode, Self::lda),
                // (_, 0xC, _, 1) => self.decode_addressing::<Read>(opcode, Self::cmp),
                // (_, 0xE, _, 1) => self.decode_addressing::<Read>(opcode, Self::sbc),

                // RMW
                // (_, 0x0, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::asl),
                // (_, 0x2, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::rol),
                // (_, 0x4, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::lsr),
                // (_, 0x6, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::ror),
                // (_, 0x8, 0xA, _) => self.decode_addressing::<Read>(opcode, Self::txa),
                // (_, 0x8, 0x1A, _) => self.decode_addressing::<Read>(opcode, Self::txs),
                (_, 0x8, _, 2) => self.decode_addressing::<Write>(opcode, Self::stx),
                // (_, 0xA, 0xA, _) => self.decode_addressing::<Read>(opcode, Self::tax),
                // (_, 0xA, 0x1A, _) => self.decode_addressing::<Read>(opcode, Self::tsx),
                (_, 0xA, _, 2) => self.decode_addressing::<Read>(opcode, Self::ldx),
                // (_, 0xC, 0xA, _) => self.decode_addressing::<Read>(opcode, Self::dex),
                // (_, 0xC, _, 2) => self.decode_addressing::<ReadWrite>(opcode, Self::dec),
                (_, 0xE, 0xA, _) => self.decode_addressing::<Read>(opcode, Self::nop),
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

        self.queue_microcode(Self::pc_inc, BusDirection::Read, Self::pull_operand);

        if should_branch {
            self.queue_microcode(Self::pc, BusDirection::Read, |cpu, _| {
                let mut pc = cpu.pc;
                pc.offset(cpu.operand.0 as i8);

                if cpu.pc.high() != pc.high() {
                    cpu.push_microcode(Self::pc_offset_wrapping, BusDirection::Read, |cpu, _| {
                        cpu.pc.offset(cpu.operand.0 as i8)
                    });
                } else {
                    cpu.pc = pc;
                }
            });
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
        todo!();
        // self.queue_microcode(Self::read_pc, BusDirection::Read, Self::nop);

        // match opcode {
        //     0x08 => self.queue_microcode(Self::php, BusDirection::Write, Self::nop),
        //     0x28 => {
        //         self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::nop);
        //         self.queue_microcode(Self::read_stack, BusDirection::Read, Self::plp);
        //     }
        //     0x48 => self.queue_microcode(Self::pha, BusDirection::Write, Self::nop),
        //     0x68 => {
        //         self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::nop);
        //         self.queue_microcode(Self::read_stack, BusDirection::Read, Self::pla);
        //     }
        //     _ => unreachable!(),
        // }

        // self.queue_decode();
    }

    fn read_stack(&mut self) {
        self.bus_address = Address(0x100 | self.stack as u16);
    }

    fn stack_pull(&mut self) {
        self.bus_address = Address(0x100 | self.stack as u16);
        self.stack = self.stack.wrapping_add(1);
    }

    fn push_pch(&mut self, data: &mut u8) {
        *data = self.pc.high();
    }

    fn push_pcl(&mut self, data: &mut u8) {
        *data = self.pc.low();
    }

    fn push_p(&mut self, data: &mut u8) {
        *data = self.p.union(StatusFlags::STACK_MASK.complement()).bits();
    }

    fn pull_pch(&mut self, data: &mut u8) {
        self.pc.set_high(*data);
    }

    fn pull_pcl(&mut self, data: &mut u8) {
        self.pc.set_low(*data);
    }

    fn pull_p(&mut self, data: &mut u8) {
        self.p.remove(StatusFlags::STACK_MASK);
        self.p
            .insert(StatusFlags::STACK_MASK.intersection(StatusFlags::from_bits_retain(*data)));
    }

    fn pull_operand(&mut self, data: &mut u8) {
        self.operand.0 = *data;
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

    fn nop(&mut self, _: &mut u8) {}

    fn bit(&mut self) {
        let result = self.a & self.bus_data;
        let flags = StatusFlags::N | StatusFlags::V;
        self.p.remove(flags);
        self.p.insert(StatusFlags::from_bits_retain(
            (flags.bits() & self.bus_data),
        ));
        self.p.set(StatusFlags::Z, result == 0);
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

    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.set_value_flags(self.y);
    }

    fn tay(&mut self) {
        self.y = self.a;
        self.set_value_flags(self.y);
    }

    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.set_value_flags(self.y);
    }

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.set_value_flags(self.x);
    }

    fn jmp(&mut self, data: &mut u8) {
        self.pc = Address((*data as u16) << 8 | self.operand.0 as u16);
    }

    fn clc(&mut self) {
        self.p.set(StatusFlags::C, false);
    }

    fn sec(&mut self, _: &mut u8) {
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

    fn tya(&mut self) {
        self.a = self.y;
        self.set_value_flags(self.a);
    }

    fn sty(&mut self) {
        self.bus_data = self.y;
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

    fn asl(&mut self) {
        self.p.set(StatusFlags::C, self.bus_data & 0b1000_0000 > 0);
        self.bus_data = self.bus_data << 1;
        self.set_value_flags(self.bus_data);
    }

    fn rol(&mut self) {
        let bit0 = if self.p.contains(StatusFlags::C) {
            0b1
        } else {
            0
        };
        self.p.set(StatusFlags::C, self.bus_data & 0b1000_0000 > 0);
        self.bus_data = (self.bus_data << 1) | bit0;
        self.set_value_flags(self.bus_data);
    }

    fn lsr(&mut self) {
        self.p.set(StatusFlags::C, self.bus_data & 1 > 0);
        self.bus_data = self.bus_data >> 1;
        self.set_value_flags(self.bus_data);
    }

    fn ror(&mut self) {
        let bit7 = if self.p.contains(StatusFlags::C) {
            0b1000_0000
        } else {
            0
        };
        self.p.set(StatusFlags::C, self.bus_data & 1 > 0);
        self.bus_data = (self.bus_data >> 1) | bit7;
        self.set_value_flags(self.bus_data);
    }

    fn txa(&mut self) {
        self.a = self.x;
        self.set_value_flags(self.a);
    }

    fn txs(&mut self) {
        self.stack = self.x;
    }

    fn stx(&mut self, data: &mut u8) {
        *data = self.x;
    }

    fn tax(&mut self) {
        self.x = self.a;
        self.set_value_flags(self.x);
    }

    fn tsx(&mut self) {
        self.x = self.stack;
        self.set_value_flags(self.x);
    }

    fn ldx(&mut self, data: &mut u8) {
        self.x = *data;
        self.set_value_flags(self.x);
    }

    fn dex(&mut self) {
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

    fn stack_push(&mut self) -> Address {
        let address = Address(0x100 | self.stack as u16);
        self.stack = self.stack.wrapping_sub(1);
        address
    }

    fn stack_pull(&mut self) -> Address {
        todo!()
    }

    fn vector<const VECTOR: u8>(&mut self) -> Address {
        Address(0xFF00 | VECTOR as u16)
    }

    fn zeropage(&mut self) -> Address {
        Address(self.operand.0 as u16)
    }
}

impl Cpu for RP2A03 {
    fn cycle(&mut self, bus: &mut impl Bus) {
        self.cycles += 1;
        match self.timing.pop_front().unwrap() {
            (address_mode, BusDirection::Read, operation) => {
                let mut data = bus.read(address_mode(self));
                operation(self, &mut data);
            }
            (address_mode, BusDirection::Write, operation) => {
                let mut data = 0;
                operation(self, &mut data);
                bus.write(address_mode(self), data);
            }
        }
    }

    fn push_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection,
        operation: fn(&mut Self, &mut u8),
    ) {
        self.timing.push_front((address_mode, bus_mode, operation));
    }

    fn queue_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection,
        operation: fn(&mut Self, &mut u8),
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

    fn load_accumulator(&mut self) {
        self.a = self.bus_data;
    }

    fn address_increment(&mut self) {
        self.bus_address.increment();
    }

    fn store_accumulator(&mut self) {
        self.bus_data = self.a;
    }

    fn zeropage_indexedx(&mut self) {
        self.bus_address = Address(self.operand.0.wrapping_add(self.x) as u16);
    }

    fn zeropage_indexedx_inc(&mut self) {
        self.bus_address = Address(self.operand.1.wrapping_add(self.x).wrapping_add(1) as u16);
    }

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
        todo!();
        // self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::nop);
        // self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::nop);
        // self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::plp);
        // self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::set_pcl);
        // self.queue_microcode(Self::read_stack, BusDirection::Read, Self::set_pch);
        // self.queue_decode();
    }

    fn queue_rts(&mut self) {
        todo!();
        // self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::nop);
        // self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::nop);
        // self.queue_microcode(Self::stack_pull, BusDirection::Read, Self::set_pcl);
        // self.queue_microcode(Self::read_stack, BusDirection::Read, Self::set_pch);
        // self.queue_microcode(Self::read_pc_inc, BusDirection::Read, Self::nop);
        // self.queue_decode();
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
