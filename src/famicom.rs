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
        // let fk = ((opcode & 0xF0) >> 4, opcode, opcode & 0x3);
        match ((opcode & 0xF0) >> 4, opcode, opcode & 0x3) {
            (_, 0x4C, _) => {
                self.queue_microcode(Self::read_pc, BusDirection::Read, Self::push_operand);
                self.queue_microcode(Self::read_pc, BusDirection::Read, Self::jmp);
                self.queue_microcode(Self::read_pc, BusDirection::Read, Self::decode);
            }
            (0x8, _, 2) => self.decode_addressing::<Write>(opcode, Self::stx),
            (0xA, _, 2) => self.decode_addressing::<Read>(opcode, Self::ldx),

            _ => unimplemented!("No decode for {:02X}", opcode),
        }
    }

    fn decode_addressing<IO: IOMode>(&mut self, opcode: u8, instruction: fn(&mut Self))
    where
        Absolute: AddressingMode<Self, IO>,
        Accumulator: AddressingMode<Self, IO>,
        Immediate: AddressingMode<Self, IO>,
        ZeroPage: AddressingMode<Self, IO>,
    {
        self.instruction = instruction;

        match opcode & 0x1f {
            0x00 | 0x02 => Immediate::enqueue(self),
            0x01 | 0x03 => unimplemented!("(d,x)"),
            0x04..=0x07 => ZeroPage::enqueue(self),
            0x08 | 0x0A => Accumulator::enqueue(self),
            0x09 | 0x0B => unimplemented!("#i"),
            0x0C..=0x0F => Absolute::enqueue(self),
            0x10 | 0x12 => unimplemented!("*+d"),
            0x11 | 0x13 => unimplemented!("(d),y"),
            0x14..=0x17 => unimplemented!("d,x/y"),
            0x18 | 0x1A => unimplemented!(""),
            0x19 | 0x1B => unimplemented!("a,y"),
            0x1C..=0x1F => unimplemented!("a,x"),
            _ => unreachable!(),
        }

        self.opcode = opcode;
    }

    fn nop(&mut self) {}

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

    // fn read_pc_null(&mut self) -> BusOperation {
    //     self.bus_address = self.pc;
    //     self.op = |_| {};
    //     BusOperation::Read
    //     // BusOperation::ReadNull
    // }

    // fn read_stack_null(&mut self) -> BusOperation {
    //     self.bus_address = Address(0x100 | self.stack as u16);
    //     self.stack = self.stack.wrapping_sub(1);
    //     self.op = |_| {};
    //     BusOperation::Read
    //     // BusOperation::ReadNull
    // }
    // fn read_vector_pch<const ABL: u8>(&mut self) -> BusOperation {
    //     self.bus_address = Address(0xFF00 | ABL as u16);
    //     self.op = |cpu| cpu.pc.set_high(cpu.bus_data);
    //     BusOperation::Read
    //     // BusOperation::ReadPCH
    // }

    // fn read_vector_pcl<const ABL: u8>(&mut self) -> BusOperation {
    //     self.bus_address = Address(0xFF00 | ABL as u16);
    //     self.op = |cpu| cpu.pc.set_low(cpu.bus_data);
    //     BusOperation::Read
    //     // BusOperation::ReadPCL
    // }

    // pub fn decode_opcode(opcode: u8) -> Result<mos6502::OpCode, u8> {
    //     if opcode & 0b0001_1111 == 0x10 {
    //         Self::decode_branch(opcode)
    //     } else {
    //         let category = opcode & 0b11;
    //         match category {
    //             0 => Self::decode_control(opcode),
    //             1 => Self::decode_alu(opcode),
    //             2 => Self::decode_rmw(opcode),
    //             _ => Self::decode_illegal(opcode),
    //         }
    //     }
    // }

    // fn decode_control(opcode: u8) -> Result<mos6502::OpCode, u8> {
    //     let instruction = if opcode < 0x80 {
    //         match opcode {
    //             // 0x00 => Instruction::BRK,
    //             0x4c => Instruction::JMP,
    //             0x6c => Instruction::JIA,
    //             _ => panic!("Missing control opcode {:02X}", opcode),
    //         }
    //     } else {
    //         panic!("Missing control opcode {:02X}", opcode)
    //     };

    //     Ok(match opcode >> 2 & 0xf {
    //         0 => OpCode::Immediate(instruction),
    //         1 => OpCode::ZeroPage(instruction),
    //         2 => OpCode::Implied(instruction),
    //         3 => OpCode::Absolute(instruction),
    //         // 4 => OpCode::ProgramCounterRelative(instruction),
    //         5 => OpCode::ZeroPageIndexedX(instruction),
    //         6 => OpCode::Implied(instruction),
    //         _ => OpCode::AbsoluteIndexedX(instruction),
    //     })
    // }

    // fn decode_branch(opcode: u8) -> Result<mos6502::OpCode, u8> {
    //     Err(opcode)
    // }

    // fn decode_alu(opcode: u8) -> Result<mos6502::OpCode, u8> {
    //     match opcode {
    //         _ => panic!("Missing ALU opcode {:02X}", opcode),
    //     }
    // }

    // fn decode_rmw(opcode: u8) -> Result<mos6502::OpCode, u8> {
    //     let instruction = match opcode {
    //         0xA2 => Instruction::LDX,
    //         _ => panic!("Missing RMW opcode {:02X}", opcode),
    //     };

    //     Ok(match opcode >> 2 & 0xf {
    //         0 => OpCode::Immediate(instruction),
    //         1 => OpCode::ZeroPage(instruction),
    //         // 2 => OpCode::Implied(instruction),
    //         // 3 => OpCode::Absolute(instruction),
    //         4 => OpCode::Jam,
    //         // 5 => OpCode::ZeroPageIndexedX(instruction),
    //         // 6 => OpCode::Implied(instruction),
    //         _ => OpCode::AbsoluteIndexedX(instruction),
    //     })
    // }

    // fn decode_illegal(opcode: u8) -> Result<mos6502::OpCode, u8> {
    //     Err(opcode)
    // }

    // if opcode == 0 {
    //     Ok(OpCode::Stack(Instruction::BRK))
    // } else if opcode & 0b0001_1111 == 0b0001_0000 {
    //     // Branch operations
    //     unimplemented!("Branch operation decode");
    // } else if opcode & 0b1110_0000 == 0b1010_0000 {
    //     let destination = match opcode & 0b111 {
    //         0 => Ok(Instruction::LDY),
    //         1 => Ok(Instruction::LDA),
    //         2 => Ok(Instruction::LDX),
    //         _ => Err(opcode),
    //     }?;
    //     match opcode {
    //         0xa0 => Ok(OpCode::Immediate(destination)),
    //         0xa1 => Ok(OpCode::AbsoluteIndexedX(destination)),
    //         0xa2 => Ok(OpCode::Immediate(destination)),
    //         0xa4 => Ok(OpCode::ZeroPage(destination)),
    //         0xa5 => Ok(OpCode::ZeroPage(destination)),
    //         0xa6 => Ok(OpCode::ZeroPage(destination)),
    //         0xa8 => Ok(OpCode::Implied(destination)),
    //         0xa9 => Ok(OpCode::Immediate(destination)),
    //         0xaa => Ok(OpCode::Implied(destination)),
    //         0xac => Ok(OpCode::Absolute(destination)),
    //         0xad => Ok(OpCode::Absolute(destination)),
    //         0xae => Ok(OpCode::Absolute(destination)),

    //         0xb1 => Ok(OpCode::AbsoluteIndexedY(destination)),
    //         0xb4 => Ok(OpCode::ZeroPageIndexedX(destination)),
    //         0xb5 => Ok(OpCode::ZeroPageIndexedX(destination)),
    //         0xb6 => Ok(OpCode::ZeroPageIndexedY(destination)),
    //         0xb9 => Ok(OpCode::AbsoluteIndexedY(destination)),
    //         0xba => Ok(OpCode::Implied(destination)),
    //         0xbc => Ok(OpCode::AbsoluteIndexedX(destination)),
    //         0xbd => Ok(OpCode::AbsoluteIndexedX(destination)),
    //         0xbe => Ok(OpCode::AbsoluteIndexedY(destination)),
    //         _ => Err(opcode),
    //     }
    // } else {
    //     match opcode {
    //         0x00 => Ok(OpCode::Stack(Instruction::BRK)),
    //         0x4C => Ok(OpCode::Absolute(Instruction::JMP)),
    //         _ => Err(opcode),
    //     }
    // }
    // }

    fn set_value_flags(&mut self, value: u8) {
        self.p.set(StatusFlags::Z, value == 0);
        self.p.set(StatusFlags::N, value > 0x80);
    }

    fn jmp(&mut self) {
        self.pc = Address((self.bus_data as u16) << 8 | self.operand.0 as u16);
    }

    fn ldx(&mut self) {
        self.x = self.bus_data;
        self.set_value_flags(self.x);
    }

    fn stx(&mut self) {
        self.bus_data = self.x;
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
