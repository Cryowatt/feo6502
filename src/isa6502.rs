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
    fn pc(&mut self) -> Address;
    fn pc_inc(&mut self) -> Address;
    fn pc_offset_wrapping(&mut self) -> Address;
    fn stack(&mut self) -> Address;
    fn stack_push(&mut self) -> Address;
    fn stack_pull(&mut self) -> Address;
    fn vector<const VECTOR: u8>(&mut self) -> Address;
    fn zeropage(&mut self) -> Address;
}

pub trait Cpu {
    fn cycle(&mut self, bus: &mut impl Bus);
    fn push_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection,
        operation: fn(&mut Self, &mut u8),
    );
    fn queue_microcode(
        &mut self,
        address_mode: fn(&mut Self) -> Address,
        bus_mode: BusDirection,
        operation: fn(&mut Self, &mut u8),
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
    fn load_accumulator(&mut self);
    fn store_accumulator(&mut self);

    // fn decode(&mut self);
    // fn zeropage(&mut self);
    fn zeropage_indexedx(&mut self);
    fn zeropage_indexedx_inc(&mut self);
    fn address_increment(&mut self);

    // fn read_pc_inc(&mut self) -> BusMode;
    // fn decode(&mut self) -> BusMode;
    // fn read_operand(&mut self) -> BusMode;
    // fn read_operand_execute(&mut self) -> BusMode;
}

pub mod addressing {
    use crate::BusDirection;

    use super::{AddressMode, Cpu};

    pub struct Read;
    pub struct ReadWrite;
    pub struct Write;

    pub trait IOMode {}

    impl IOMode for Read {}
    impl IOMode for ReadWrite {}
    impl IOMode for Write {}

    pub trait AddressingMode<Cpu, Mode: IOMode> {
        fn enqueue(cpu: &mut Cpu);
    }

    pub struct Immediate;

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Read> for Immediate {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read, CPU::instruction);
            cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for Immediate {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for Immediate {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    pub struct IndexedIndirectX;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for IndexedIndirectX {
        fn enqueue(cpu: &mut CPU) {
            todo!();
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            // cpu.queue_microcode(CPU::zeropage, BusDirection::Read, CPU::nop);
            // cpu.queue_microcode(
            //     CPU::zeropage_indexedx,
            //     BusDirection::Read,
            //     CPU::push_operand,
            // );
            // cpu.queue_microcode(
            //     CPU::zeropage_indexedx_inc,
            //     BusDirection::Read,
            //     CPU::address_operand,
            // );
            // cpu.queue_microcode(CPU::nop, BusDirection::Read, CPU::instruction);
            // cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for IndexedIndirectX {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for IndexedIndirectX {
        fn enqueue(cpu: &mut CPU) {
            todo!();
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            // cpu.queue_microcode(CPU::zeropage, BusDirection::Read, CPU::nop);
            // cpu.queue_microcode(
            //     CPU::zeropage_indexedx,
            //     BusDirection::Read,
            //     CPU::push_operand,
            // );
            // cpu.queue_microcode(
            //     CPU::zeropage_indexedx_inc,
            //     BusDirection::Read,
            //     CPU::address_operand,
            // );
            // cpu.queue_microcode(CPU::instruction, BusDirection::Write, CPU::nop);
            // cpu.queue_decode();
        }
    }

    pub struct ZeroPage;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for ZeroPage {
        fn enqueue(cpu: &mut CPU) {
            todo!();

            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            // cpu.queue_microcode(CPU::zeropage, BusDirection::Read, CPU::instruction);
            // cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for ZeroPage {
        fn enqueue(cpu: &mut CPU) {
            todo!();
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            // cpu.queue_microcode(CPU::zeropage, BusDirection::Read, CPU::nop);
            // cpu.queue_microcode(CPU::zeropage, BusDirection::Write, CPU::instruction);
            // cpu.queue_microcode(CPU::zeropage, BusDirection::Write, CPU::nop);
            // cpu.queue_decode();
        }
    }

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Write> for ZeroPage {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::pc_inc, BusDirection::Read, CPU::pull_operand);
            cpu.queue_microcode(CPU::zeropage, BusDirection::Write, CPU::instruction);
            cpu.queue_decode();
        }
    }

    pub struct Accumulator;

    impl<CPU: Cpu + AddressMode> AddressingMode<CPU, Read> for Accumulator {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::pc, BusDirection::Read, |cpu, data| {
                cpu.store_accumulator();
                cpu.instruction(data);
            });
            cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for Accumulator {
        fn enqueue(cpu: &mut CPU) {
            todo!()
            // cpu.queue_microcode(CPU::nop, BusDirection::Read, |cpu| {
            //     cpu.store_accumulator();
            //     cpu.instruction();
            //     cpu.load_accumulator();
            // });
            // cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for Accumulator {
        fn enqueue(cpu: &mut CPU) {
            todo!()
            // cpu.queue_microcode(CPU::nop, BusDirection::Read, |cpu| cpu.bus_data = cpu.a);
            // cpu.queue_decode();
        }
    }

    pub struct Absolute;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for Absolute {
        fn enqueue(cpu: &mut CPU) {
            todo!();
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::address_operand);
            // cpu.queue_microcode(CPU::nop, BusDirection::Read, CPU::instruction);
            // cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for Absolute {
        fn enqueue(cpu: &mut CPU) {
            todo!();
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::address_operand);
            // cpu.queue_microcode(CPU::nop, BusDirection::Read, CPU::nop);
            // cpu.queue_microcode(CPU::nop, BusDirection::Write, CPU::instruction);
            // cpu.queue_microcode(CPU::nop, BusDirection::Write, CPU::nop);
            // cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for Absolute {
        fn enqueue(cpu: &mut CPU) {
            todo!();
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            // cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::address_operand);
            // cpu.queue_microcode(CPU::instruction, BusDirection::Write, CPU::nop);
            // cpu.queue_decode();
        }
    }

    pub struct IndirectIndexedY;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for IndirectIndexedY {
        fn enqueue(cpu: &mut CPU) {
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
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for IndirectIndexedY {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    pub struct Implied;

    impl<CPU: Cpu + AddressMode, MODE: IOMode> AddressingMode<CPU, MODE> for Implied {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::pc, BusDirection::Read, CPU::instruction);
            cpu.queue_decode();
        }
    }
}
