use addressing::*;
use bitflags::bitflags;
use instructions::{Instruction, MicrocodeControl, ReadInstruction, WriteInstruction};

use crate::{Address, Bus};

pub mod addressing;
pub mod instructions;

#[derive(Debug, Default)]
pub struct Registers {
    pub pc: Address,
    pub stack: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: StatusFlags,
    pub address_buffer: Address,
    pub operand: u8,
}

#[derive(Debug)]
pub enum BusDirection<CPU> {
    Write(fn(&mut CPU)),
    Read(fn(&mut CPU)),
}

pub trait MicrocodeInstructions {
    fn pull_operand(&mut self);
    fn index_x(&self) -> u8;
    fn index_y(&self) -> u8;
    fn buffer_low(&mut self);
    fn buffer_high(&mut self);
    fn read_instruction<INST: ReadInstruction>(&mut self);
    fn write_instruction<INST: WriteInstruction>(&mut self);
    fn borrow_accumulator(&mut self) -> &mut u8;
    fn nop(&mut self) {}
}

pub trait Decode: MicrocodeControl + AddressMode {
    fn decode_opcode(&mut self);
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
        Self: Sized;
    fn queue_branch(&mut self);
    fn queue_brk(&mut self);
    fn queue_jsr(&mut self);
    fn queue_rti(&mut self);
    fn queue_rts(&mut self);
    fn queue_jmp(&mut self);
    fn queue_indirect_jmp(&mut self);
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct StatusFlags : u8{
        // NV1BDIZC
        const C = 0b0000_0001;
        const Z = 0b0000_0010;
        const I = 0b0000_0100;
        const D = 0b0000_1000;
        const B = 0b0001_0000;
        const Reserved = 0b0010_0000;
        const STACK_MASK = 0b1100_1111;
        const V = 0b0100_0000;
        const N = 0b1000_0000;
    }
}

impl Default for StatusFlags {
    fn default() -> Self {
        Self::Reserved | Self::I
    }
}

impl StatusFlags {
    fn set_value_flags(&mut self, value: u8) {
        self.set(StatusFlags::Z, value == 0);
        self.set(StatusFlags::N, (value as i8) < 0);
    }
}

pub trait AddressMode {
    fn address(&mut self) -> Address;
    fn pc(&mut self) -> Address;
    fn pc_inc(&mut self) -> Address;
    fn stack(&mut self) -> Address;
    fn stack_push(&mut self) -> Address;
    fn stack_pull(&mut self) -> Address;
    fn vector<const VECTOR: u8>(&mut self) -> Address;
    fn zeropage(&mut self) -> Address;
}

pub trait Cpu
where
    Self: Sized,
{
    const CLOCK_DIVISOR: u64;
    fn cycle(&mut self, bus: &mut impl Bus);
}
