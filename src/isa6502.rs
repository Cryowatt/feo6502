use bitflags::bitflags;

use crate::{Address, Bus, BusDirection};

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct StatusFlags:u8{
        // NV1BDIZC
        const C = 0b0000_0001;
        const Z = 0b0000_0010;
        const I = 0b0000_0100;
        const D = 0b0000_1000;
        const B = 0b0001_0000;
        const Default = 0b0010_0100;
        const STACK_MASK = 0b1100_1111;
        const V = 0b0100_0000;
        const N = 0b1000_0000;
    }
}

pub trait AddressMode {
    fn address(&mut self) -> Address;
    fn address_indexedx(&mut self) -> Address;
    fn address_inc(&mut self) -> Address;
    fn buffer(&mut self, address: Address) -> Address;
    fn pc(&mut self) -> Address;
    fn pc_inc(&mut self) -> Address;
    fn pc_offset_wrapping(&mut self) -> Address;
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
    fn cycle(&mut self, bus: &mut impl Bus);
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
    fn queue_decode(&mut self);
    fn queue_jsr(&mut self);
    fn queue_jmp(&mut self);
    fn queue_rti(&mut self);
    fn queue_rts(&mut self);
    fn address_operand(&mut self, data: &mut u8);
    fn read_pc(&mut self);
    fn read_pc_inc(&mut self);
    fn pull_operand(&mut self, data: &mut u8);
    fn instruction(&mut self, data: &mut u8);
    fn with_accumulator(&mut self, operation: fn(&mut Self, data: &mut u8));
    // fn load_accumulator(&mut self);
    // fn store_accumulator(&mut self);

    // fn decode(&mut self);
    // fn zeropage(&mut self);
    // fn zeropage_indexedx(&mut self);
    // fn zeropage_indexedx_inc(&mut self);
    // fn address_increment(&mut self);

    // fn read_pc_inc(&mut self) -> BusMode;
    // fn decode(&mut self) -> BusMode;
    // fn read_operand(&mut self) -> BusMode;
    // fn read_operand_execute(&mut self) -> BusMode;
}

pub mod addressing {
    use crate::{Address, BusDirection};

    use super::{AddressMode, Cpu};

    pub struct Read;
    pub struct ReadWrite;
    pub struct Write;

    pub trait IOMode<CPU> {
        type Instruction;
    }

    impl<CPU> IOMode<CPU> for Read {
        type Instruction = fn(&mut CPU, u8);
    }
    impl<CPU> IOMode<CPU> for ReadWrite {
        type Instruction = fn(&mut CPU, u8) -> u8;
    }
    impl<CPU> IOMode<CPU> for Write {
        type Instruction = fn(&mut CPU) -> u8;
    }

    fn nop<CPU>(_: &mut CPU, _: &mut u8) {}
    fn buffer<CPU: AddressMode>(cpu: &mut CPU, address: fn(&mut CPU) -> Address) -> Address {
        let address = address(cpu);
        cpu.buffer(address);
        address
    }

    pub trait AddressingMode<Cpu, Mode: IOMode<Cpu>> {
        fn enqueue(cpu: &mut Cpu, instruction: Mode::Instruction);
    }

    pub struct Immediate;

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Read> for Immediate {
        fn enqueue(cpu: &mut CPU, instruction: <Read as IOMode<CPU>>::Instruction) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(instruction));
            cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for Immediate {
        fn enqueue(cpu: &mut CPU, instruction: <ReadWrite as IOMode<CPU>>::Instruction) {
            todo!()
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for Immediate {
        fn enqueue(cpu: &mut CPU, instruction: <Write as IOMode<CPU>>::Instruction) {
            todo!()
        }
    }

    pub struct IndexedIndirectX;

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Read> for IndexedIndirectX {
        fn enqueue(cpu: &mut CPU, instruction: <Read as IOMode<CPU>>::Instruction) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
            cpu.queue_microcode(|cpu| buffer(cpu, CPU::zeropage), BusDirection::Read(nop));
            cpu.queue_microcode(
                |cpu| buffer(cpu, CPU::address_indexedx),
                BusDirection::Read(CPU::pull_operand),
            );
            cpu.queue_microcode(CPU::address_inc, BusDirection::Read(CPU::address_operand));
            cpu.queue_microcode(CPU::address, BusDirection::Read(instruction));
            cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for IndexedIndirectX {
        fn enqueue(cpu: &mut CPU, instruction: <ReadWrite as IOMode<CPU>>::Instruction) {
            todo!()
        }
    }

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Write> for IndexedIndirectX {
        fn enqueue(cpu: &mut CPU, instruction: <Write as IOMode<CPU>>::Instruction) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
            cpu.queue_microcode(|cpu| buffer(cpu, CPU::zeropage), BusDirection::Read(nop));
            cpu.queue_microcode(
                |cpu| buffer(cpu, CPU::address_indexedx),
                BusDirection::Read(CPU::pull_operand),
            );
            cpu.queue_microcode(CPU::address_inc, BusDirection::Read(CPU::address_operand));
            cpu.queue_microcode(CPU::address, BusDirection::Write(instruction));
            cpu.queue_decode();
        }
    }

    pub struct ZeroPage;

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Read> for ZeroPage {
        fn enqueue(cpu: &mut CPU, instruction: <Read as IOMode<CPU>>::Instruction) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
            cpu.queue_microcode(CPU::zeropage, BusDirection::Read(instruction));
            cpu.queue_decode();
        }
    }

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, ReadWrite> for ZeroPage {
        fn enqueue(cpu: &mut CPU, instruction: <ReadWrite as IOMode<CPU>>::Instruction) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
            cpu.queue_microcode(CPU::zeropage, BusDirection::Read(CPU::latch_data));
            cpu.queue_microcode(CPU::zeropage, BusDirection::Write(nop));
            cpu.queue_microcode(CPU::zeropage, BusDirection::Write(instruction));
            cpu.queue_decode();
            todo!();
        }
    }

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Write> for ZeroPage {
        fn enqueue(cpu: &mut CPU, instruction: <Write as IOMode<CPU>>::Instruction) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
            cpu.queue_microcode(CPU::zeropage, BusDirection::Write(instruction));
            cpu.queue_decode();
        }
    }

    pub struct Accumulator;

    impl<CPU: Cpu + AddressMode, IOMODE: IOMode<CPU>> AddressingMode<CPU, IOMODE> for Accumulator {
        fn enqueue(cpu: &mut CPU, instruction: IOMODE::Instruction) {
            cpu.queue_microcode(
                CPU::pc,
                BusDirection::Read(|cpu, _| {
                    cpu.with_accumulator(Cpu::instruction);
                }),
            );
            cpu.queue_decode();
        }
    }

    pub struct Absolute;

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Read> for Absolute {
        fn enqueue(cpu: &mut CPU, instruction: <Read as IOMode<CPU>>::Instruction) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::address_operand));
            cpu.queue_microcode(CPU::address, BusDirection::Read(instruction));
            cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for Absolute {
        fn enqueue(cpu: &mut CPU, instruction: <ReadWrite as IOMode<CPU>>::Instruction) {
            todo!();
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::address_operand);
            // cpu.queue_microcode(CPU::nop, BusDirection::Read, CPU::nop);
            // cpu.queue_microcode(CPU::nop, BusDirection::Write, CPU::instruction);
            // cpu.queue_microcode(CPU::nop, BusDirection::Write, CPU::nop);
            // cpu.queue_decode();
        }
    }

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Write> for Absolute {
        fn enqueue(cpu: &mut CPU, instruction: <Write as IOMode<CPU>>::Instruction) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::pull_operand));
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read(CPU::address_operand));
            cpu.queue_microcode(CPU::address, BusDirection::Write(instruction));
            cpu.queue_decode();
        }
    }

    pub struct IndirectIndexedY;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for IndirectIndexedY {
        fn enqueue(cpu: &mut CPU, instruction: <Read as IOMode<CPU>>::Instruction) {
            todo!();
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::nop);
            // cpu.queue_microcode(CPU::zeropage, BusDirection::Read, CPU::address_operand);
            // cpu.queue_microcode(
            //     CPU::address_increment,
            //     BusDirection::Read,
            //     CPU::address_operand,
            // );
            // cpu.queue_microcode(
            //     |cpu| cpu.zeropage_indexedx()
            //     CPU::ad,
            //     BusDirection::Read,
            //     CPU::address_operand,
            // );
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for IndirectIndexedY {
        fn enqueue(cpu: &mut CPU, instruction: <ReadWrite as IOMode<CPU>>::Instruction) {
            todo!()
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for IndirectIndexedY {
        fn enqueue(cpu: &mut CPU, instruction: <Write as IOMode<CPU>>::Instruction) {
            todo!()
        }
    }

    pub struct Implied;

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Read> for Implied {
        fn enqueue(cpu: &mut CPU, instruction: <Read as IOMode<CPU>>::Instruction) {
            cpu.queue_microcode(CPU::pc, BusDirection::Read(instruction));
            cpu.queue_decode();
        }
    }
}
