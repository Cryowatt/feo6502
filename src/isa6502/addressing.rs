use core::marker::PhantomData;

use crate::{Address, BusDirection};

use super::{
    instructions::{
        Instruction, MicrocodeControl, ReadInstruction, ReadWriteInstruction, WriteInstruction,
    },
    AddressMode, MicrocodeInstructions, Registers,
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

pub trait AddressingMode<CPU: MicrocodeControl + AddressMode, INST: Instruction<IO>, IO: IOMode> {
    fn enqueue(cpu: &mut CPU);
}

pub struct Addressing;

pub struct Immediate;

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for Immediate
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_read::<INST>(CPU::pc_inc);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for Immediate
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::nop));
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for Immediate
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::nop));
        cpu.queue_decode();
    }
}

pub struct IndexedIndirectX;
impl IndexedIndirectX {
    fn zeropage_indexed_low<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions>(
        cpu: &mut CPU,
    ) -> Address {
        cpu.zeropage().index(cpu.index_x())
    }

    fn zeropage_indexed_high<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions>(
        cpu: &mut CPU,
    ) -> Address {
        cpu.zeropage().index(cpu.index_x().wrapping_add(1))
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for IndexedIndirectX
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(
            Self::zeropage_indexed_low,
            BusDirection::Read(CPU::buffer_low),
        );
        cpu.queue_microcode(
            Self::zeropage_indexed_high,
            BusDirection::Read(CPU::buffer_high),
        );
        cpu.queue_read::<INST>(CPU::address);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for IndexedIndirectX
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(
            Self::zeropage_indexed_low,
            BusDirection::Read(CPU::buffer_low),
        );
        cpu.queue_microcode(
            Self::zeropage_indexed_high,
            BusDirection::Read(CPU::buffer_high),
        );
        cpu.queue_microcode(CPU::address, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(CPU::address, BusDirection::Write(CPU::nop));
        cpu.queue_read_write::<INST>(CPU::address);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for IndexedIndirectX
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(
            Self::zeropage_indexed_low,
            BusDirection::Read(CPU::buffer_low),
        );
        cpu.queue_microcode(
            Self::zeropage_indexed_high,
            BusDirection::Read(CPU::buffer_high),
        );
        cpu.queue_write::<INST>(CPU::address);
        cpu.queue_decode();
    }
}

pub struct ZeroPage;

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for ZeroPage
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_read::<INST>(CPU::zeropage);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadWriteInstruction>
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

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for ZeroPage
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_write::<INST>(CPU::zeropage);
        cpu.queue_decode();
    }
}

pub struct Stack;

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for Stack
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(CPU::stack, BusDirection::Read(CPU::nop));
        cpu.queue_read::<INST>(CPU::stack_pull);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for Stack
{
    fn enqueue(_cpu: &mut CPU) {
        todo!()
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for Stack
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc, BusDirection::Read(CPU::nop));
        cpu.queue_write::<INST>(CPU::stack_push);
        cpu.queue_decode();
    }
}

pub struct Accumulator;

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadInstruction>
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

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for Accumulator
{
    fn enqueue(cpu: &mut CPU) {
        pub struct WithAccumulator<INST> {
            _inst: PhantomData<INST>,
        }

        impl<INST: ReadWriteInstruction> ReadInstruction for WithAccumulator<INST> {
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

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: WriteInstruction>
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

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for Absolute
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_high));
        cpu.queue_read::<INST>(CPU::address);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadWriteInstruction>
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

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: WriteInstruction>
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
impl IndirectIndexedY {
    fn zeropage_high<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions>(
        cpu: &mut CPU,
    ) -> Address {
        cpu.zeropage().index(1)
    }

    fn address_indexed_y_no_carry<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions>(
        cpu: &mut CPU,
    ) -> Address {
        cpu.address().index(cpu.index_y())
    }

    fn address_indexed_y<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions>(
        cpu: &mut CPU,
    ) -> Address {
        cpu.address() + cpu.index_y()
    }

    fn buffer_high_maybe_pagefix<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions>(
        cpu: &mut CPU,
    ) {
        cpu.buffer_high();
        let address = cpu.address();
        let indexed_address = address.index(cpu.index_y());
        let fixed_adddress = address + cpu.index_y();

        // Maybe inject invalid address
        if indexed_address != fixed_adddress {
            cpu.push_microcode(
                Self::address_indexed_y_no_carry,
                BusDirection::Read(CPU::nop),
            );
        }
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for IndirectIndexedY
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(
            Self::zeropage_high,
            BusDirection::Read(Self::buffer_high_maybe_pagefix),
        );
        cpu.queue_read::<INST>(Self::address_indexed_y);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for IndirectIndexedY
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(Self::zeropage_high, BusDirection::Read(CPU::buffer_high));
        cpu.queue_microcode(
            Self::address_indexed_y_no_carry,
            BusDirection::Read(CPU::nop),
        );
        cpu.queue_microcode(Self::address_indexed_y, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(Self::address_indexed_y, BusDirection::Write(CPU::nop));
        cpu.queue_read_write::<INST>(Self::address_indexed_y);
        cpu.queue_decode();
    }
}

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for IndirectIndexedY
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(Self::zeropage_high, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(
            Self::address_indexed_y_no_carry,
            BusDirection::Read(CPU::nop),
        );
        cpu.queue_write::<INST>(Self::address_indexed_y);
        cpu.queue_decode();
    }
}

pub struct ZeroPageIndexed<const INDEX_X: bool>;

impl<const INDEX_X: bool> ZeroPageIndexed<INDEX_X> {
    fn zeropage_indexed<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions>(
        cpu: &mut CPU,
    ) -> Address {
        cpu.zeropage().index(if INDEX_X {
            cpu.index_x()
        } else {
            cpu.index_y()
        })
    }
}

impl<
        CPU: MicrocodeControl + AddressMode + MicrocodeInstructions,
        INST: ReadInstruction,
        const INDEX_X: bool,
    > AddressingMode<CPU, INST, Read> for ZeroPageIndexed<INDEX_X>
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::nop));
        cpu.queue_read::<INST>(Self::zeropage_indexed);
        cpu.queue_decode();
    }
}

impl<
        CPU: MicrocodeControl + AddressMode + MicrocodeInstructions,
        INST: ReadWriteInstruction,
        const INDEX_X: bool,
    > AddressingMode<CPU, INST, ReadWrite> for ZeroPageIndexed<INDEX_X>
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(Self::zeropage_indexed, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(Self::zeropage_indexed, BusDirection::Write(CPU::nop));
        cpu.queue_read_write::<INST>(Self::zeropage_indexed);
        cpu.queue_decode();
    }
}

impl<
        CPU: MicrocodeControl + AddressMode + MicrocodeInstructions,
        INST: WriteInstruction,
        const INDEX_X: bool,
    > AddressingMode<CPU, INST, Write> for ZeroPageIndexed<INDEX_X>
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
        cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::nop));
        cpu.queue_write::<INST>(Self::zeropage_indexed);
        cpu.queue_decode();
    }
}

pub struct Implied;

impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadInstruction>
    AddressingMode<CPU, INST, Read> for Implied
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_read::<INST>(CPU::pc);
        cpu.queue_decode();
    }
}

// Acts as a NOP
impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: ReadWriteInstruction>
    AddressingMode<CPU, INST, ReadWrite> for Implied
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc, BusDirection::Read(CPU::nop));
        cpu.queue_decode();
    }
}

// Acts as a NOP
impl<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions, INST: WriteInstruction>
    AddressingMode<CPU, INST, Write> for Implied
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc, BusDirection::Read(CPU::nop));
        cpu.queue_decode();
    }
}

pub struct AbsoluteIndexed<const INDEX_X: bool>;
impl<const INDEX_X: bool> AbsoluteIndexed<INDEX_X> {
    fn address_indexed<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions>(
        cpu: &mut CPU,
    ) -> Address {
        cpu.address().index(if INDEX_X {
            cpu.index_x()
        } else {
            cpu.index_y()
        })
    }

    fn address_indexed_corrected<CPU: MicrocodeControl + AddressMode + MicrocodeInstructions>(
        cpu: &mut CPU,
    ) -> Address {
        cpu.address()
            + if INDEX_X {
                cpu.index_x()
            } else {
                cpu.index_y()
            }
    }
}

impl<
        CPU: MicrocodeControl + AddressMode + MicrocodeInstructions,
        INST: ReadInstruction,
        const INDEX_X: bool,
    > AddressingMode<CPU, INST, Read> for AbsoluteIndexed<INDEX_X>
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(
            CPU::pc_inc,
            BusDirection::Read(|cpu| {
                cpu.buffer_high();
                let address = cpu.address();
                if INDEX_X {
                    let indexed_address = address.index(cpu.index_x());
                    let fixed_adddress = address + cpu.index_x();

                    if indexed_address != fixed_adddress {
                        cpu.push_microcode(
                            |cpu| cpu.address().index(cpu.index_x()),
                            BusDirection::Read(CPU::nop),
                        );
                    }
                } else {
                    let indexed_address = address.index(cpu.index_y());
                    let fixed_adddress = address + cpu.index_y();

                    if indexed_address != fixed_adddress {
                        cpu.push_microcode(
                            |cpu| cpu.address().index(cpu.index_y()),
                            BusDirection::Read(CPU::nop),
                        );
                    }
                }
            }),
        );
        cpu.queue_read::<INST>(Self::address_indexed_corrected);
        cpu.queue_decode();
    }
}

impl<
        CPU: MicrocodeControl + AddressMode + MicrocodeInstructions,
        INST: ReadWriteInstruction,
        const INDEX_X: bool,
    > AddressingMode<CPU, INST, ReadWrite> for AbsoluteIndexed<INDEX_X>
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_high));
        cpu.queue_microcode(Self::address_indexed, BusDirection::Read(CPU::nop));
        cpu.queue_microcode(
            Self::address_indexed_corrected,
            BusDirection::Read(CPU::nop),
        );
        cpu.queue_microcode(
            Self::address_indexed_corrected,
            BusDirection::Write(CPU::nop),
        );
        cpu.queue_read_write::<INST>(Self::address_indexed_corrected);
        cpu.queue_decode();
    }
}

impl<
        CPU: MicrocodeControl + AddressMode + MicrocodeInstructions,
        INST: WriteInstruction,
        const INDEX_X: bool,
    > AddressingMode<CPU, INST, Write> for AbsoluteIndexed<INDEX_X>
{
    fn enqueue(cpu: &mut CPU) {
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_low));
        cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::buffer_high));
        cpu.queue_microcode(Self::address_indexed, BusDirection::Read(CPU::nop));
        cpu.queue_write::<INST>(Self::address_indexed_corrected);
        cpu.queue_decode();
    }
}
