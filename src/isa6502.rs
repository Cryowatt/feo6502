use bitflags::bitflags;

use crate::{Bus, BusDirection};

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

pub trait Cpu {
    fn cycle(&mut self, bus: &mut impl Bus);
    fn queue_microcode(
        &mut self,
        pre_bus: fn(&mut Self),
        bus_mode: BusDirection,
        post_bus: fn(&mut Self),
    );
    fn queue_decode(&mut self);
    fn queue_jsr(&mut self);
    fn queue_jmp(&mut self);
    fn queue_rti(&mut self);
    fn queue_rts(&mut self);
    fn nop(&mut self);
    fn address_operand(&mut self);
    fn read_pc(&mut self);
    fn read_pc_inc(&mut self);
    fn push_operand(&mut self);
    fn instruction(&mut self);
    fn instruction_write(&mut self);
    fn decode(&mut self);
    fn zeropage(&mut self);

    // fn read_pc_inc(&mut self) -> BusMode;
    // fn decode(&mut self) -> BusMode;
    // fn read_operand(&mut self) -> BusMode;
    // fn read_operand_execute(&mut self) -> BusMode;
}

pub mod addressing {
    use crate::BusDirection;

    use super::Cpu;

    pub struct Read;
    pub struct ReadWrite;
    pub struct Write;

    pub trait IOMode {}

    impl IOMode for Read {}
    impl IOMode for ReadWrite {}
    impl IOMode for Write {}

    fn nop<CPU: Cpu>(_: &mut CPU) {}

    pub trait AddressingMode<Cpu, Mode: IOMode> {
        fn enqueue(cpu: &mut Cpu);
    }

    pub struct Absolute;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for Absolute {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::address_operand);
            cpu.queue_microcode(CPU::nop, BusDirection::Read, CPU::instruction);
            cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for Absolute {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for Absolute {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::push_operand);
            cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::address_operand);
            cpu.queue_microcode(CPU::instruction, BusDirection::Write, CPU::nop);
            cpu.queue_decode();
        }
    }

    pub struct Accumulator;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for Accumulator {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for Accumulator {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for Accumulator {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    pub struct Immediate;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for Immediate {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::instruction);
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
            cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::instruction);
            cpu.queue_decode();
        }
    }

    pub struct ZeroPage;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for ZeroPage {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::zeropage);
            cpu.queue_microcode(CPU::nop, BusDirection::Read, CPU::instruction);
            cpu.queue_decode();
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, ReadWrite> for ZeroPage {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    impl<CPU: Cpu> AddressingMode<CPU, Write> for ZeroPage {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::read_pc_inc, BusDirection::Read, CPU::zeropage);
            cpu.queue_microcode(CPU::instruction, BusDirection::Write, nop);
            cpu.queue_decode();
        }
    }

    pub struct Implied;

    impl<CPU: Cpu, MODE: IOMode> AddressingMode<CPU, MODE> for Implied {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::nop, BusDirection::Read, CPU::instruction);
            cpu.queue_decode();
        }
    }
}
