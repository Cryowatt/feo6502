use std::collections::VecDeque;

use crate::{
    isa6502::{addressing::*, instructions::*, *},
    *,
};

pub mod mapper;
pub mod rom;

type Microcode<CPU> = (fn(&mut CPU) -> Address, BusDirection<CPU>);

#[derive(Debug)]
pub struct RP2A03 {
    registers: Registers,
    decode_cache: [Option<fn(&mut Self)>; 256],
    timing: VecDeque<Microcode<Self>>,
    opcode: u8,
    data_latch: u8,
    cycles: u64,
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

    fn pc(&mut self) -> Address {
        self.registers.pc
    }

    fn pc_inc(&mut self) -> Address {
        let address = self.pc();
        self.registers.pc.increment();
        address
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

        if let Some(enqueue) = self.decode_cache[self.opcode as usize] {
            enqueue(self);
            return;
        }

        // 0000_0000
        // bit 7-5: row
        // bit 4-0: column
        // bit 1-0: block
        let row = (self.opcode & 0b1110_0000) >> 4;
        let column = self.opcode & 0b0001_1111;
        let block = self.opcode & 0b0000_0011;

        let enqueue_timing: fn(&mut Self) = if self.opcode & 0x1F == 0x10 {
            Self::queue_branch
        } else {
            match (row, column, block) {
                // Control
                (0x2, 0x0, _) => Self::queue_brk,
                (0x2, 0x0, _) => Self::queue_jsr,
                (0x4, 0x0, _) => Self::queue_rti,
                (0x6, 0x0, _) => Self::queue_rts,
                (0x2, 0x4, _) => self.decode_addressing::<BIT, Read>(row, column),
                (0x0, 0x8, _) => self.decode_addressing::<PHP, Write>(row, column),
                (0x2, 0x8, _) => self.decode_addressing::<PLP, Read>(row, column),
                (0x4, 0x8, _) => self.decode_addressing::<PHA, Write>(row, column),
                (0x6, 0x8, _) => self.decode_addressing::<PLA, Read>(row, column),
                (0x8, 0x8, _) => self.decode_addressing::<DEY, Read>(row, column),
                (0xA, 0x8, _) => self.decode_addressing::<TAY, Read>(row, column),
                (0xC, 0x8, _) => self.decode_addressing::<INY, Read>(row, column),
                (0xE, 0x8, _) => self.decode_addressing::<INX, Read>(row, column),
                (0x2, 0xC, _) => self.decode_addressing::<BIT, Read>(row, column),
                (0x4, 0xC, _) => Self::queue_jmp,
                (0x6, 0xC, _) => Self::queue_indirect_jmp,
                (0x0, 0x18, _) => self.decode_addressing::<CLC, Read>(row, column),
                (0x2, 0x18, _) => self.decode_addressing::<SEC, Read>(row, column),
                (0x6, 0x18, _) => self.decode_addressing::<SEI, Read>(row, column),
                (0x8, 0x18, _) => self.decode_addressing::<TYA, Read>(row, column),
                (0xA, 0x18, _) => self.decode_addressing::<CLV, Read>(row, column),
                (0xC, 0x18, _) => self.decode_addressing::<CLD, Read>(row, column),
                (0xE, 0x18, _) => self.decode_addressing::<SED, Read>(row, column),

                (0x8, _, 0) => self.decode_addressing::<STY, Write>(row, column),
                (0xA, _, 0) => self.decode_addressing::<LDY, Read>(row, column),
                (_, 0x14, _) => self.decode_addressing::<NOP, Read>(row, column),
                (_, 0x1C, _) => self.decode_addressing::<NOP, Read>(row, column),
                (0xC, _, 0) => self.decode_addressing::<CPY, Read>(row, column),
                (0xE, _, 0) => self.decode_addressing::<CPX, Read>(row, column),
                (_, _, 0) => self.decode_addressing::<NOP, Read>(row, column),

                // ALU
                (0x0, _, 1) => self.decode_addressing::<ORA, Read>(row, column),
                (0x2, _, 1) => self.decode_addressing::<AND, Read>(row, column),
                (0x4, _, 1) => self.decode_addressing::<EOR, Read>(row, column),
                (0x6, _, 1) => self.decode_addressing::<ADC<false>, Read>(row, column),
                (0x8, _, 1) => self.decode_addressing::<STA, Write>(row, column),
                (0xA, _, 1) => self.decode_addressing::<LDA, Read>(row, column),
                (0xC, _, 1) => self.decode_addressing::<CMP, Read>(row, column),
                (0xE, _, 1) => self.decode_addressing::<SBC, Read>(row, column),

                // RMW
                (0x0, _, 2) => self.decode_addressing::<ASL, ReadWrite>(row, column),
                (0x2, _, 2) => self.decode_addressing::<ROL, ReadWrite>(row, column),
                (0x4, _, 2) => self.decode_addressing::<LSR, ReadWrite>(row, column),
                (0x6, _, 2) => self.decode_addressing::<ROR, ReadWrite>(row, column),
                (0x8, 0xA, _) => self.decode_addressing::<TXA, Read>(row, column),
                (0x8, 0x1A, _) => self.decode_addressing::<TXS, Read>(row, column),
                (0x8, _, 2) => self.decode_addressing::<STX, Write>(row, column),
                (0xA, 0xA, _) => self.decode_addressing::<TAX, Read>(row, column),
                (0xA, 0x1A, _) => self.decode_addressing::<TSX, Read>(row, column),
                (0xA, _, 2) => self.decode_addressing::<LDX, Read>(row, column),
                (0xC, 0xA, _) => self.decode_addressing::<DEX, Read>(row, column),
                (0xC, _, 2) => self.decode_addressing::<DEC, ReadWrite>(row, column),
                (0xE, 0xA, _) => self.decode_addressing::<NOP, Read>(row, column),
                (0xE, _, 2) => self.decode_addressing::<INC, ReadWrite>(row, column),

                // Illegal
                (0x0, _, 3) => self.decode_addressing::<SLO, ReadWrite>(row, column),
                (0x2, _, 3) => self.decode_addressing::<RLA, ReadWrite>(row, column),
                (0x4, _, 3) => self.decode_addressing::<SRE, ReadWrite>(row, column),
                (0x6, _, 3) => self.decode_addressing::<RRA, ReadWrite>(row, column),
                (0x8, _, 3) => self.decode_addressing::<SAX, Write>(row, column),
                (0xA, _, 3) => self.decode_addressing::<LAX, Read>(row, column),
                (0xC, _, 3) => self.decode_addressing::<DCP, ReadWrite>(row, column),
                (0xE, 0xB, _) => self.decode_addressing::<SBC, Read>(row, column),
                (0xE, _, 3) => self.decode_addressing::<ISC, ReadWrite>(row, column),
                _ => unimplemented!("No decode for {:02X}", self.opcode),
            }
        };
        enqueue_timing(self);
        self.decode_cache[self.opcode as usize] = Some(enqueue_timing);
    }

    fn decode_addressing<INST: Instruction<IO>, IO: IOMode>(
        &mut self,
        row: u8,
        column: u8,
    ) -> fn(&mut Self)
    where
        Immediate: AddressingMode<Self, INST, IO>,
        IndexedIndirectX: AddressingMode<Self, INST, IO>,
        ZeroPage: AddressingMode<Self, INST, IO>,
        Stack: AddressingMode<Self, INST, IO>,
        Accumulator: AddressingMode<Self, INST, IO>,
        Absolute: AddressingMode<Self, INST, IO>,
        IndirectIndexedY: AddressingMode<Self, INST, IO>,
        ZeroPageIndexed<true>: AddressingMode<Self, INST, IO>,
        ZeroPageIndexed<false>: AddressingMode<Self, INST, IO>,
        Implied: AddressingMode<Self, INST, IO>,
        AbsoluteIndexed<true>: AddressingMode<Self, INST, IO>,
        AbsoluteIndexed<false>: AddressingMode<Self, INST, IO>,
    {
        match column {
            0x00 | 0x02 => Self::addressing::<Immediate, INST, IO>,
            0x01 | 0x03 => Self::addressing::<IndexedIndirectX, INST, IO>,
            0x04..=0x07 => Self::addressing::<ZeroPage, INST, IO>,
            0x08 => match row {
                0x0..=0x6 => Self::addressing::<Stack, INST, IO>,
                _ => Self::addressing::<Accumulator, INST, IO>,
            },
            0x0A => Self::addressing::<Accumulator, INST, IO>,
            0x09 | 0x0B => Self::addressing::<Immediate, INST, IO>,
            0x0C..=0x0F => Self::addressing::<Absolute, INST, IO>,
            0x11 | 0x13 => Self::addressing::<IndirectIndexedY, INST, IO>,
            0x14 | 0x15 => Self::addressing::<ZeroPageIndexed<true>, INST, IO>,
            0x16 | 0x17 => match row {
                0x8 | 0xA => Self::addressing::<ZeroPageIndexed<false>, INST, IO>,
                _ => Self::addressing::<ZeroPageIndexed<true>, INST, IO>,
            },
            0x18 | 0x1A => Self::addressing::<Implied, INST, IO>,
            0x19 | 0x1B => Self::addressing::<AbsoluteIndexed<false>, INST, IO>,
            0x1C | 0x1D => Self::addressing::<AbsoluteIndexed<true>, INST, IO>,
            0x1E | 0x1F => match row {
                0x8 | 0xA => Self::addressing::<AbsoluteIndexed<false>, INST, IO>,
                _ => Self::addressing::<AbsoluteIndexed<true>, INST, IO>,
            },
            _ => unreachable!("No addressing mode implemented for {:02X}", column),
        }
    }

    fn queue_branch(&mut self) {
        let should_branch = match self.opcode {
            0x10 => !self.registers.p.contains(StatusFlags::N),
            0x30 => self.registers.p.contains(StatusFlags::N),
            0x50 => !self.registers.p.contains(StatusFlags::V),
            0x70 => self.registers.p.contains(StatusFlags::V),
            0x90 => !self.registers.p.contains(StatusFlags::C),
            0xB0 => self.registers.p.contains(StatusFlags::C),
            0xD0 => !self.registers.p.contains(StatusFlags::Z),
            0xF0 => self.registers.p.contains(StatusFlags::Z),
            _ => unreachable!("{:02X}", self.opcode),
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
                            |cpu| {
                                let mut address = cpu.registers.pc;
                                address.offset(cpu.registers.operand as i8);
                                address.set_high(cpu.registers.pc.high());
                                address
                            },
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

    fn queue_brk(&mut self) {
        todo!()
    }

    fn queue_jmp(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::pull_operand));
        self.queue_read::<JMP>(Self::pc_inc);
        self.queue_decode();
    }

    fn queue_indirect_jmp(&mut self) {
        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::buffer_low));
        self.queue_microcode(Self::pc_inc, BusDirection::Read(Self::buffer_high));
        self.queue_microcode(Self::address, BusDirection::Read(Self::pull_operand));
        self.queue_read::<JMP>(|cpu| cpu.address().index(1));
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

impl MicrocodeInstructions for RP2A03 {
    fn pull_operand(&mut self) {
        self.registers.operand = self.data_latch;
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
            timing: VecDeque::with_capacity(8),
            decode_cache: [None; 256],
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
    const CLOCK_DIVISOR: u64 = 12;

    fn cycle(&mut self, bus: &mut impl Bus) {
        self.cycles = self.cycles.wrapping_add(1);
        if self.cycles % Self::CLOCK_DIVISOR == 0 {
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
    pub cycles: u64,
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
