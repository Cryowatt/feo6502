use std::collections::VecDeque;

use crate::{
    isa6502::{addressing::*, instructions::*, *},
    *,
};

pub mod mapper;
pub mod rom;

#[derive(Debug)]
pub struct RP2A03 {
    registers: Registers,
    timing: VecDeque<(fn(&mut Self) -> Address, BusDirection<Self>)>,
    opcode: u8,
    data_latch: u8,
    cycles: u32,
}

impl MicrocodeControl for RP2A03 {
    fn push_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection<Self>,
    ) {
        self.timing.push_front((address_mode, bus_mode));
    }

    fn queue_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection<Self>,
    ) {
        self.timing.push_back((address_mode, bus_mode));
    }

    fn queue_decode(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::decode_opcode));
    }

    fn clear_microcode(&mut self) {
        self.timing.clear();
    }

    fn queue_read<INST: ReadInstruction>(&mut self, address_mode: fn(&mut Self) -> Address) {
        self.queue_microcode(
            address_mode,
            BusDirection::Read(|cpu| INST::execute(&mut cpu.registers, &cpu.data_latch)),
        );
    }

    fn queue_read_write<INST: ReadWriteInstruction>(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
    ) {
        self.queue_microcode(
            address_mode,
            BusDirection::Write(|cpu| INST::execute(&mut cpu.registers, &mut cpu.data_latch)),
        );
    }

    fn queue_write<INST: WriteInstruction>(&mut self, address_mode: fn(&mut Self) -> Address) {
        self.queue_microcode(
            address_mode,
            BusDirection::Write(|cpu| INST::execute(&mut cpu.registers, &mut cpu.data_latch)),
        );
    }
}

impl AddressMode for RP2A03 {
    fn address(&mut self) -> Address {
        self.registers.address_buffer
    }

    fn address_indexedx(&mut self) -> Address {
        todo!()
    }

    fn address_inc(&mut self) -> Address {
        todo!()
    }

    fn buffer(&mut self, address: Address) -> Address {
        todo!()
    }

    fn pc(&mut self) -> Address {
        self.registers.pc
    }

    fn pc_inc(&mut self) -> Address {
        let address = self.pc();
        self.registers.pc.increment();
        address
    }

    fn pc_offset_wrapping(&mut self) -> Address {
        todo!()
    }

    fn stack(&mut self) -> Address {
        Address(0x100 | self.registers.stack as u16)
    }

    fn stack_push(&mut self) -> Address {
        let address = self.stack();
        self.registers.stack = self.registers.stack.wrapping_sub(1);
        address
    }

    fn stack_pull(&mut self) -> Address {
        self.registers.stack = self.registers.stack.wrapping_add(1);
        self.stack()
    }

    fn vector<const VECTOR: u8>(&mut self) -> Address {
        Address(0xFF00 | VECTOR as u16)
    }

    fn zeropage(&mut self) -> Address {
        Address(self.registers.operand as u16)
    }
}

impl Decode for RP2A03 {
    fn decode_opcode(&mut self) {
        self.opcode = self.data_latch;
        // 0000_0000
        // bit 7: high/low
        // bit 7-5: row
        // bit 4-0: column
        // bit 1-0: block
        let high = (self.opcode & 0b1000_0000) > 0;
        let row = (self.opcode & 0b1110_0000) >> 4;
        let column = self.opcode & 0b0001_1111;
        let block = self.opcode & 0b0000_0011;

        // println!("{} {:X} {:X} {}", high, row, column, block);
        if self.opcode & 0x1F == 0x10 {
            self.decode_branch(self.opcode);
        } else {
            match (high, row, column, block) {
                // Control
                (_, 0x2, 0x0, _) => self.queue_jsr(),
                (_, 0x4, 0x0, _) => self.queue_rti(),
                (_, 0x6, 0x0, _) => self.queue_rts(),
                (_, 0x2, 0x4, _) => self.decode_addressing::<BIT, Read>(column),
                (false, _, 0x8, _) => self.decode_stack(row),
                (_, 0x8, 0x8, _) => self.decode_addressing::<DEY, Read>(column),
                // (_, 0xA, 0x8, _) => self.decode_addressing::<Read>(opcode, Self::tay),
                (_, 0xC, 0x8, _) => self.decode_addressing::<INY, Read>(column),
                (_, 0xE, 0x8, _) => self.decode_addressing::<INX, Read>(column),
                (_, 0x2, 0xC, _) => self.decode_addressing::<BIT, Read>(column),
                (_, 0x4, 0xC, _) => self.queue_jmp(),
                (_, 0x0, 0x18, _) => self.decode_addressing::<CLC, Read>(column),
                (_, 0x2, 0x18, _) => self.decode_addressing::<SEC, Read>(column),
                (_, 0x6, 0x18, _) => self.decode_addressing::<SEI, Read>(column),
                (_, 0xA, 0x18, _) => self.decode_addressing::<CLV, Read>(column),
                (_, 0xC, 0x18, _) => self.decode_addressing::<CLD, Read>(column),
                (_, 0xE, 0x18, _) => self.decode_addressing::<SED, Read>(column),

                (_, 0x8, 0x18, _) => self.decode_addressing::<TYA, Read>(column),
                (_, 0x8, _, 0) => self.decode_addressing::<STY, Write>(column),
                (_, 0xA, _, 0) => self.decode_addressing::<LDY, Read>(column),
                (_, 0xC, _, 0) => self.decode_addressing::<CPY, Read>(column),
                (_, 0xE, _, 0) => self.decode_addressing::<CPX, Read>(column),

                // ALU
                (_, 0x0, _, 1) => self.decode_addressing::<ORA, Read>(column),
                (_, 0x2, _, 1) => self.decode_addressing::<AND, Read>(column),
                (_, 0x4, _, 1) => self.decode_addressing::<EOR, Read>(column),
                (_, 0x6, _, 1) => self.decode_addressing::<ADC<false>, Read>(column),
                (_, 0x8, _, 1) => self.decode_addressing::<STA, Write>(column),
                (_, 0xA, _, 1) => self.decode_addressing::<LDA, Read>(column),
                (_, 0xC, _, 1) => self.decode_addressing::<CMP, Read>(column),
                (_, 0xE, _, 1) => self.decode_addressing::<SBC, Read>(column),

                // RMW
                (_, 0x0, _, 2) => self.decode_addressing::<ASL, ReadWrite>(column),
                (_, 0x2, _, 2) => self.decode_addressing::<ROL, ReadWrite>(column),
                (_, 0x4, _, 2) => self.decode_addressing::<LSR, ReadWrite>(column),
                (_, 0x6, _, 2) => self.decode_addressing::<ROR, ReadWrite>(column),
                (_, 0x8, 0xA, _) => self.decode_addressing::<TXA, Read>(column),
                (_, 0x8, 0x1A, _) => self.decode_addressing::<TXS, Read>(column),
                (_, 0x8, _, 2) => self.decode_addressing::<STX, Write>(column),
                // (_, 0xA, 0xA, _) => self.decode_addressing::<Read>(opcode, Self::tax),
                (_, 0xA, 0x1A, _) => self.decode_addressing::<TSX, Read>(column),
                (_, 0xA, _, 2) => self.decode_addressing::<LDX, Read>(column),
                (_, 0xC, 0xA, _) => self.decode_addressing::<DEX, Read>(column),
                (_, 0xC, _, 2) => self.decode_addressing::<DEC, ReadWrite>(column),
                (_, 0xE, 0xA, _) => self.decode_addressing::<NOP, Read>(column),
                (_, 0xE, _, 2) => self.decode_addressing::<INC, ReadWrite>(column),

                // Illegal
                _ => unimplemented!("No decode for {:02X}", self.opcode),
            }
        }
    }

    fn decode_addressing<INST: Instruction<IO>, IO: IOMode>(&mut self, column: u8)
    where
        Immediate: AddressingMode<Self, INST, IO>,
        IndexedIndirectX: AddressingMode<Self, INST, IO>,
        ZeroPage: AddressingMode<Self, INST, IO>,
        Accumulator: AddressingMode<Self, INST, IO>,
        IndirectIndexedY: AddressingMode<Self, INST, IO>,
        Implied: AddressingMode<Self, INST, IO>,
        Absolute: AddressingMode<Self, INST, IO>,
        //     ZeroPageIndexed
        //     AbsoluteIndexed
    {
        match column {
            0x00 | 0x02 => self.addressing::<Immediate, INST, IO>(),
            0x01 | 0x03 => self.addressing::<IndexedIndirectX, INST, IO>(),
            0x04..=0x07 => self.addressing::<ZeroPage, INST, IO>(),
            0x08 | 0x0A => self.addressing::<Accumulator, INST, IO>(),
            0x09 | 0x0B => self.addressing::<Immediate, INST, IO>(),
            0x0C..=0x0F => self.addressing::<Absolute, INST, IO>(),
            // 0x10 | 0x12 => unimplemented!("*+d"),
            0x11 | 0x13 => self.addressing::<IndirectIndexedY, INST, IO>(),
            // 0x14..=0x17 => unimplemented!("d,x/y"),
            0x18 | 0x1A => self.addressing::<Implied, INST, IO>(),
            // 0x19 | 0x1B => unimplemented!("a,y"),
            // 0x1C..=0x1F => unimplemented!("a,x"),
            _ => unreachable!("No addressing mode implemented for {:02X}", column),
        }
    }

    fn decode_branch(&mut self, opcode: u8) {
        let should_branch = match opcode {
            0x10 => !self.registers.p.contains(StatusFlags::N),
            0x30 => self.registers.p.contains(StatusFlags::N),
            0x50 => !self.registers.p.contains(StatusFlags::V),
            0x70 => self.registers.p.contains(StatusFlags::V),
            0x90 => !self.registers.p.contains(StatusFlags::C),
            0xB0 => self.registers.p.contains(StatusFlags::C),
            0xD0 => !self.registers.p.contains(StatusFlags::Z),
            0xF0 => self.registers.p.contains(StatusFlags::Z),
            _ => todo!("{:02X}", opcode),
        };

        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::pull_operand));

        if should_branch {
            self.queue_microcode(
                Self::pc,
                BusDirection::Read(|cpu| {
                    let mut pc = cpu.registers.pc;
                    pc.offset(cpu.registers.operand as i8);

                    if cpu.registers.pc.high() != pc.high() {
                        cpu.push_microcode(
                            Self::pc_offset_wrapping,
                            BusDirection::Read(|cpu| {
                                cpu.registers.pc.offset(cpu.registers.operand as i8)
                            }),
                        );
                    } else {
                        cpu.registers.pc = pc;
                    }
                }),
            );
        }

        self.queue_decode();
    }

    fn decode_stack(&mut self, row: u8) {
        self.queue_microcode(Self::pc, BusDirection::Read(Self::nop));

        match row {
            0x0 => self.queue_write::<PHP>(Self::stack_push),
            0x2 => {
                self.queue_microcode(Self::stack, BusDirection::Read(Self::nop));
                self.queue_read::<PLP>(Self::stack_pull);
            }
            0x4 => self.queue_write::<PHA>(Self::stack_push),
            0x6 => {
                self.queue_microcode(Self::stack, BusDirection::Read(Self::nop));
                self.queue_read::<PLA>(Self::stack_pull);
            }
            _ => unreachable!(),
        }

        self.queue_decode();
    }

    fn queue_jmp(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::pull_operand));
        self.queue_read::<JMP>(Self::pc_inc);
        self.queue_decode();
    }

    fn queue_jsr(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::pull_operand));
        self.queue_microcode(Self::stack, BusDirection::Read(Self::nop));
        self.queue_microcode(
            Self::stack_push,
            BusDirection::Write(Self::write_instruction::<PCH>),
        );
        self.queue_microcode(
            Self::stack_push,
            BusDirection::Write(Self::write_instruction::<PCL>),
        );
        self.queue_read::<JSR>(Self::pc_inc);
        self.queue_decode();
    }

    fn queue_rti(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::nop));
        self.queue_microcode(Self::stack, BusDirection::Read(Self::nop));
        self.queue_read::<PLP>(Self::stack_pull);
        self.queue_read::<PCL>(Self::stack_pull);
        self.queue_read::<PCH>(Self::stack_pull);
        self.queue_decode();
    }

    fn queue_rts(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::nop));
        self.queue_microcode(Self::stack, BusDirection::Read(Self::nop));
        self.queue_read::<PCL>(Self::stack_pull);
        self.queue_read::<PCH>(Self::stack_pull);
        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::nop));
        self.queue_decode();
    }
}

impl Microcode for RP2A03 {
    fn pull_operand(&mut self) {
        self.registers.operand = self.data_latch;
    }

    fn address_operand(&mut self) {
        unimplemented!()
        // Address((self.data_latch as u16) << 8 | self.registers.operand as u16);
    }

    fn read_instruction<INST: ReadInstruction>(&mut self) {
        INST::execute(&mut self.registers, &self.data_latch);
    }

    fn write_instruction<INST: WriteInstruction>(&mut self) {
        INST::execute(&mut self.registers, &mut self.data_latch);
    }

    fn borrow_accumulator(&mut self) -> &mut u8 {
        &mut self.registers.a
    }

    fn index_x(&self) -> u8 {
        self.registers.x
    }

    fn index_y(&self) -> u8 {
        self.registers.y
    }

    fn buffer_low(&mut self) {
        self.registers.address_buffer.set_low(self.data_latch);
    }

    fn buffer_high(&mut self) {
        self.registers.address_buffer.set_high(self.data_latch);
    }
}

impl RP2A03 {
    pub fn new() -> Self {
        let mut cpu = Self {
            registers: Registers::new(),
            timing: VecDeque::with_capacity(7),
            opcode: 0,
            data_latch: 0,
            cycles: 0,
        };
        cpu.reset();
        cpu
    }

    fn addressing<ADDRESSING: AddressingMode<Self, INST, IO>, INST: Instruction<IO>, IO: IOMode>(
        &mut self,
    ) {
        ADDRESSING::enqueue(self);
    }

    pub fn reset(&mut self) {
        self.registers.stack = 0;
        self.registers.p.set(StatusFlags::Default, true);
        self.clear_microcode();
        self.queue_read::<NOP>(Self::pc_inc);
        self.queue_read::<NOP>(Self::pc_inc);
        self.queue_read::<NOP>(Self::stack_push);
        self.queue_read::<NOP>(Self::stack_push);
        self.queue_read::<NOP>(Self::stack_push);
        self.queue_read::<PCL>(Self::vector::<0xFC>);
        self.queue_read::<PCH>(Self::vector::<0xFD>);
        self.queue_decode();
    }
}

impl Cpu for RP2A03 {
    fn cycle(&mut self, bus: &mut impl Bus) {
        self.cycles += 1;
        match self.timing.pop_front().unwrap() {
            (address_mode, BusDirection::Read(operation)) => {
                self.data_latch = bus.read(address_mode(self));
                operation(self);
            }
            (address_mode, BusDirection::Write(operation)) => {
                let address = address_mode(self);
                operation(self);
                bus.write(address, self.data_latch);
            }
        }
    }
}

pub trait NesLogger {
    fn log(&self) -> NesTestLogEntry;
}

impl<Mapper: BusDevice> NesLogger for System<RP2A03, Mapper> {
    fn log(&self) -> NesTestLogEntry {
        NesTestLogEntry {
            pc: self.cpu.registers.pc,
            opcode: self.cpu.opcode,
            a: self.cpu.registers.a,
            x: self.cpu.registers.x,
            y: self.cpu.registers.y,
            p: self.cpu.registers.p.bits(),
            stack: self.cpu.registers.stack,
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
