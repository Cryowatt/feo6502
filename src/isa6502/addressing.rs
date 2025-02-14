use std::marker::PhantomData;

use crate::{Address, BusDirection};

use super::{
    instructions::{
        Instruction, MicrocodeControl, ReadInstruction, ReadInstructions, ReadWriteInstruction,
        WriteInstruction,
    },
    AddressMode, Microcode, Registers,
};

pub struct Read;
pub struct ReadWrite;
pub struct Write;

pub trait IOMode {
    type Instruction;
}

impl IOMode for Read {
    type Instruction = fn(&mut Registers, u8);
}
impl IOMode for ReadWrite {
    type Instruction = fn(&mut Registers, u8) -> u8;
}
impl IOMode for Write {
    type Instruction = fn(&mut Registers) -> u8;
}

fn buffer<CPU: AddressMode>(cpu: &mut CPU, address: fn(&mut CPU) -> Address) -> Address {
    let address = address(cpu);
    cpu.buffer(address);
    address
}

pub trait AddressingMode<CPU: MicrocodeControl + AddressMode, INST: Instruction<IO>, IO: IOMode> {
    fn enqueue(cpu: &mut CPU);
}

pub struct Addressing;

pub struct Immediate;

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for Immediate
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_read::<INST>(CPU::pc_inc);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for Immediate
{
    fn enqueue(cpu: &mut CPU) {
        todo!()
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for Immediate
{
    fn enqueue(cpu: &mut CPU) {
        todo!()
    }
}

pub struct IndexedIndirectX;

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for IndexedIndirectX
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(
            |cpu| {
                let mut address = cpu.zeropage();
                address.index(cpu.index_x())
            },
            BusDirection::Read(CPU::buffer_low),
        );
        cpu.queue_microcode(
            |cpu| {
                let mut address = cpu.zeropage();
                address.index(cpu.index_x().wrapping_add(1))
            },
            BusDirection::Read(CPU::buffer_high),
        );
        cpu.queue_read::<INST>(CPU::address);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for IndexedIndirectX
{
    fn enqueue(cpu: &mut CPU) {
        todo!()
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for IndexedIndirectX
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(
            |cpu| {
                let mut address = cpu.zeropage();
                address.index(cpu.index_x())
            },
            BusDirection::Read(CPU::buffer_low),
        );
        cpu.queue_microcode(
            |cpu| {
                let mut address = cpu.zeropage();
                address.index(cpu.index_x().wrapping_add(1))
            },
            BusDirection::Read(CPU::buffer_high),
        );
        cpu.queue_write::<INST>(CPU::address);
        cpu.queue_decode();
    }
}

pub struct ZeroPage;

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for ZeroPage
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_read::<INST>(CPU::zeropage);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for ZeroPage
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Write(CPU::nop));
        cpu.queue_read_write::<INST>(CPU::zeropage);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for ZeroPage
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_write::<INST>(CPU::zeropage);
        cpu.queue_decode();
    }
}

pub struct Accumulator;

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for Accumulator
{
    fn enqueue(cpu: &mut CPU) {
        pub struct WithAccumulator<INST> {
            _inst: PhantomData<INST>,
        }

        impl<INST: ReadInstruction> ReadInstruction for WithAccumulator<INST> {
            fn execute(registers: &mut Registers, _: &u8) {
                let data = registers.a;
                INST::execute(registers, &data);
            }
        }

        cpu.queue_read::<WithAccumulator<INST>>(CPU::pc);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for Accumulator
{
    fn enqueue(cpu: &mut CPU) {
        pub struct WithAccumulator<INST> {
            _inst: PhantomData<INST>,
        }

        impl<INST: ReadWriteInstruction> ReadWriteInstruction for WithAccumulator<INST> {
            fn execute(registers: &mut Registers, _: &mut u8) {
                let mut data = registers.a;
                INST::execute(registers, &mut data);
                registers.a = data;
            }
        }

        cpu.queue_read_write::<WithAccumulator<INST>>(CPU::pc);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for Accumulator
{
    fn enqueue(cpu: &mut CPU) {
        pub struct WithAccumulator<INST> {
            _inst: PhantomData<INST>,
        }

        impl<INST: WriteInstruction> ReadInstruction for WithAccumulator<INST> {
            fn execute(registers: &mut Registers, _: &u8) {
                let mut data = registers.a;
                INST::execute(registers, &mut data);
                registers.a = data;
            }
        }

        cpu.queue_read::<WithAccumulator<INST>>(CPU::pc);
        cpu.queue_decode();
    }
}

pub struct Absolute;

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for Absolute
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_high));
        cpu.queue_read::<INST>(CPU::address);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for Absolute
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_high));
        cpu.queue_microcode(CPU::address, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(CPU::address, BusDirection::Write(CPU::nop));
        cpu.queue_read_write::<INST>(CPU::address);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for Absolute
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_high));
        cpu.queue_write::<INST>(CPU::address);
        cpu.queue_decode();
    }
}

pub struct IndirectIndexedY;

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for IndirectIndexedY
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(
            |cpu| cpu.zeropage().index(1),
            BusDirection::Read(|cpu| {
                cpu.buffer_high();
                let address = cpu.address();
                let indexed_address = address.index(cpu.index_y());
                let fixed_adddress = address + cpu.index_y();

                // Maybe inject invalid address
                if indexed_address != fixed_adddress {
                    cpu.push_microcode(
                        |cpu| cpu.address().index(cpu.index_y()),
                        BusDirection::Read(CPU::nop),
                    );
                }
            }),
        );
        cpu.queue_read::<INST>(|cpu| cpu.address() + cpu.index_y());
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for IndirectIndexedY
{
    fn enqueue(cpu: &mut CPU) {
        todo!()
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for IndirectIndexedY
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(
            |cpu| cpu.address().index(cpu.index_y()),
            BusDirection::Read(CPU::nop),
        );
        cpu.queue_microcode(|cpu| cpu.zeropage().index(1), BusDirection::Read(CPU::nop));
        cpu.queue_write::<INST>(|cpu| cpu.address() + cpu.index_y());
        cpu.queue_decode();
    }
}

pub struct Implied;

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for Implied
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_read::<INST>(CPU::pc);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for Implied
{
    fn enqueue(cpu: &mut CPU) {
        todo!()
    }
}

impl<CPU: MicrocodeControl + AddressMode + Microcode, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for Implied
{
    fn enqueue(cpu: &mut CPU) {
        // cpu.queue_microcode(CPU::pc, BusDirection::Read());
        // cpu.queue_decode();

        todo!()
    }
}
