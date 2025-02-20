use crate::{Address, BusDirection};

use super::{
    addressing::{IOMode, Read, ReadWrite, Write},
    Registers, StatusFlags,
};

pub trait Instruction<IO>
where
    IO: IOMode,
{
}

pub trait ReadInstruction: Instruction<Read> {
    fn execute(registers: &mut Registers, data: &u8);
}

impl<T: ReadInstruction> Instruction<Read> for T {}

pub trait ReadWriteInstruction: Instruction<ReadWrite> {
    fn execute(registers: &mut Registers, data: &mut u8);
}

impl<T: ReadWriteInstruction> Instruction<ReadWrite> for T {}

pub trait WriteInstruction: Instruction<Write> {
    fn execute(registers: &mut Registers, data: &mut u8);
}

impl<T: WriteInstruction> Instruction<Write> for T {}

pub struct JSR;
impl ReadInstruction for JSR {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.pc = Address::new(*data, registers.operand);
    }
}

pub struct BIT;
impl ReadInstruction for BIT {
    fn execute(registers: &mut Registers, data: &u8) {
        let result = registers.a & data;
        let flags = StatusFlags::N | StatusFlags::V;
        registers.p.remove(flags);
        registers
            .p
            .insert(StatusFlags::from_bits_retain(flags.bits() & data));
        registers.p.set(StatusFlags::Z, result == 0);
    }
}

pub struct PHP;
impl WriteInstruction for PHP {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = (registers.p | !StatusFlags::STACK_MASK).bits();
    }
}

pub struct PLP;
impl ReadInstruction for PLP {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.p = (StatusFlags::from_bits_retain(*data) & StatusFlags::STACK_MASK)
            | (registers.p & !StatusFlags::STACK_MASK);
    }
}

pub struct PHA;
impl WriteInstruction for PHA {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = registers.a;
    }
}

pub struct PLA;
impl ReadInstruction for PLA {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.a = *data;
        registers.p.set_value_flags(registers.a);
    }
}

pub struct DEY;
impl ReadInstruction for DEY {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.y = registers.y.wrapping_sub(1);
        registers.p.set_value_flags(registers.y);
    }
}

pub struct TAY;
impl ReadInstruction for TAY {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.y = registers.a;
        registers.p.set_value_flags(registers.y);
    }
}

pub struct INY;
impl ReadInstruction for INY {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.y = registers.y.wrapping_add(1);
        registers.p.set_value_flags(registers.y);
    }
}

pub struct INX;
impl ReadInstruction for INX {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.x = registers.x.wrapping_add(1);
        registers.p.set_value_flags(registers.x);
    }
}

pub struct JMP;
impl ReadInstruction for JMP {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.pc = Address::new(*data, registers.operand);
    }
}

pub struct CLC;
impl ReadInstruction for CLC {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.p.set(StatusFlags::C, false);
    }
}

pub struct SEC;
impl ReadInstruction for SEC {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.p.set(StatusFlags::C, true);
    }
}

pub struct SEI;
impl ReadInstruction for SEI {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.p.set(StatusFlags::I, true);
    }
}

pub struct CLV;
impl ReadInstruction for CLV {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.p.set(StatusFlags::V, false);
    }
}

pub struct CLD;
impl ReadInstruction for CLD {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.p.set(StatusFlags::D, false);
    }
}

pub struct SED;
impl ReadInstruction for SED {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.p.set(StatusFlags::D, true);
    }
}

pub struct TYA;
impl ReadInstruction for TYA {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.a = registers.y;
        registers.p.set_value_flags(registers.a);
    }
}
pub struct STY;
impl WriteInstruction for STY {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = registers.y;
    }
}

pub struct LDY;
impl ReadInstruction for LDY {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.y = *data;
        registers.p.set_value_flags(registers.y);
    }
}

pub struct CPY;
impl ReadInstruction for CPY {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.p.set(StatusFlags::C, registers.y >= *data);
        registers.p.set(StatusFlags::Z, registers.y == *data);
        registers
            .p
            .set(StatusFlags::N, (registers.y.wrapping_sub(*data) as i8) < 0);
    }
}

pub struct CPX;
impl ReadInstruction for CPX {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.p.set(StatusFlags::C, registers.x >= *data);
        registers.p.set(StatusFlags::Z, registers.x == *data);
        registers
            .p
            .set(StatusFlags::N, (registers.x.wrapping_sub(*data) as i8) < 0);
    }
}

pub struct ORA;
impl ReadInstruction for ORA {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.a |= *data;
        registers.p.set_value_flags(registers.a);
    }
}

pub struct AND;
impl ReadInstruction for AND {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.a &= *data;
        registers.p.set_value_flags(registers.a);
    }
}

pub struct EOR;
impl ReadInstruction for EOR {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.a ^= data;
        registers.p.set_value_flags(registers.a);
    }
}

pub struct ADC<const ALLOW_DECIMAL: bool>;
impl ReadInstruction for ADC<false> {
    fn execute(registers: &mut Registers, data: &u8) {
        let (result, add_overflow) = registers.a.overflowing_add(*data);
        let (result, carry_overflow) = result.overflowing_add(registers.p.bits() & 1);
        registers
            .p
            .set(StatusFlags::C, add_overflow | carry_overflow);
        registers.p.set(
            StatusFlags::V,
            (result ^ registers.a) & (result ^ data) & 0x80 > 0,
        );
        registers.a = result;
        registers.p.set_value_flags(registers.a);
    }
}

pub struct STA;
impl WriteInstruction for STA {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = registers.a;
    }
}

pub struct LDA;
impl ReadInstruction for LDA {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.a = *data;
        registers.p.set_value_flags(registers.a);
    }
}

pub struct CMP;
impl ReadInstruction for CMP {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.p.set(StatusFlags::C, registers.a >= *data);
        registers.p.set(StatusFlags::Z, registers.a == *data);
        registers
            .p
            .set(StatusFlags::N, (registers.a.wrapping_sub(*data) as i8) < 0);
    }
}

pub struct SBC;
impl ReadInstruction for SBC {
    fn execute(registers: &mut Registers, data: &u8) {
        let (result, add_overflow) = registers.a.overflowing_add(!*data);
        let (result, carry_overflow) = result.overflowing_add(registers.p.bits() & 1);
        registers
            .p
            .set(StatusFlags::C, add_overflow | carry_overflow);
        registers.p.set(
            StatusFlags::V,
            (result ^ registers.a) & (result ^ !*data) & 0x80 > 0,
        );
        registers.a = result;
        registers.p.set_value_flags(registers.a);
    }
}

pub struct ASL;
impl ReadWriteInstruction for ASL {
    fn execute(registers: &mut Registers, data: &mut u8) {
        registers.p.set(StatusFlags::C, *data & 0b1000_0000 > 0);
        *data <<= 1;
        registers.p.set_value_flags(*data);
    }
}

pub struct ROL;
impl ReadWriteInstruction for ROL {
    fn execute(registers: &mut Registers, data: &mut u8) {
        let bit0 = if registers.p.contains(StatusFlags::C) {
            0b1
        } else {
            0
        };
        registers.p.set(StatusFlags::C, *data & 0b1000_0000 > 0);
        *data <<= 1;
        *data |= bit0;
        registers.p.set_value_flags(*data);
    }
}

pub struct LSR;
impl ReadWriteInstruction for LSR {
    fn execute(registers: &mut Registers, data: &mut u8) {
        registers.p.set(StatusFlags::C, *data & 1 > 0);
        *data >>= 1;
        registers.p.set_value_flags(*data);
    }
}

pub struct ROR;
impl ReadWriteInstruction for ROR {
    fn execute(registers: &mut Registers, data: &mut u8) {
        let bit7 = if registers.p.contains(StatusFlags::C) {
            0b1000_0000
        } else {
            0
        };
        registers.p.set(StatusFlags::C, *data & 1 > 0);
        *data >>= 1;
        *data |= bit7;
        registers.p.set_value_flags(*data);
    }
}

pub struct TXA;
impl ReadInstruction for TXA {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.a = registers.x;
        registers.p.set_value_flags(registers.a);
    }
}

pub struct TXS;
impl ReadInstruction for TXS {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.stack = registers.x;
    }
}

pub struct STX;
impl WriteInstruction for STX {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = registers.x;
    }
}

pub struct TAX;
impl ReadInstruction for TAX {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.x = registers.a;
        registers.p.set_value_flags(registers.x);
    }
}

pub struct TSX;
impl ReadInstruction for TSX {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.x = registers.stack;
        registers.p.set_value_flags(registers.x);
    }
}

pub struct LDX;
impl ReadInstruction for LDX {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.x = *data;
        registers.p.set_value_flags(registers.x);
    }
}

pub struct DEX;
impl ReadInstruction for DEX {
    fn execute(registers: &mut Registers, _: &u8) {
        registers.x = registers.x.wrapping_sub(1);
        registers.p.set_value_flags(registers.x);
    }
}

pub struct DEC;
impl ReadWriteInstruction for DEC {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = data.wrapping_sub(1);
        registers.p.set_value_flags(*data);
    }
}

pub struct NOP;
impl ReadInstruction for NOP {
    fn execute(_: &mut Registers, _: &u8) {}
}

pub struct INC;
impl ReadWriteInstruction for INC {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = data.wrapping_add(1);
        registers.p.set_value_flags(*data);
    }
}

// Illegal instructions
pub struct SLO;
impl ReadWriteInstruction for SLO {
    fn execute(registers: &mut Registers, data: &mut u8) {
        ASL::execute(registers, data);
        ORA::execute(registers, data);
    }
}

pub struct RLA;
impl ReadWriteInstruction for RLA {
    fn execute(registers: &mut Registers, data: &mut u8) {
        ROL::execute(registers, data);
        AND::execute(registers, data);
    }
}

pub struct SRE;
impl ReadWriteInstruction for SRE {
    fn execute(registers: &mut Registers, data: &mut u8) {
        LSR::execute(registers, data);
        EOR::execute(registers, data);
    }
}

pub struct RRA;
impl ReadWriteInstruction for RRA {
    fn execute(registers: &mut Registers, data: &mut u8) {
        ROR::execute(registers, data);
        ADC::execute(registers, data);
    }
}

pub struct SAX;
impl WriteInstruction for SAX {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = registers.a & registers.x;
    }
}

pub struct LAX;
impl ReadInstruction for LAX {
    fn execute(registers: &mut Registers, data: &u8) {
        LDA::execute(registers, data);
        TAX::execute(registers, data);
    }
}

pub struct DCP;
impl ReadWriteInstruction for DCP {
    fn execute(registers: &mut Registers, data: &mut u8) {
        DEC::execute(registers, data);
        CMP::execute(registers, data);
    }
}

pub struct ISC;
impl ReadWriteInstruction for ISC {
    fn execute(registers: &mut Registers, data: &mut u8) {
        INC::execute(registers, data);
        SBC::execute(registers, data);
    }
}

// Pseudo-instructions
pub struct PCL;
impl ReadInstruction for PCL {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.pc.set_low(*data);
    }
}

impl WriteInstruction for PCL {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = registers.pc.low()
    }
}

pub struct PCH;
impl ReadInstruction for PCH {
    fn execute(registers: &mut Registers, data: &u8) {
        registers.pc.set_high(*data);
    }
}

impl WriteInstruction for PCH {
    fn execute(registers: &mut Registers, data: &mut u8) {
        *data = registers.pc.high()
    }
}

pub trait MicrocodeControl
where
    Self: Sized,
{
    fn push_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection<Self>,
    );
    fn queue_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection<Self>,
    );
    fn queue_read<INST: ReadInstruction>(&mut self, address_mode: fn(&mut Self) -> Address);
    fn queue_read_write<INST: ReadWriteInstruction>(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
    );
    fn queue_write<INST: WriteInstruction>(&mut self, address_mode: fn(&mut Self) -> Address);
    fn queue_decode(&mut self);
    fn clear_microcode(&mut self);
}
