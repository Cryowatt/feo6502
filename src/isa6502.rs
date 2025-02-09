use bitflags::bitflags;

use crate::{Bus, BusMode};

bitflags! {
    #[derive(Debug)]
    pub struct StatusFlags:u8{
        // NV1BDIZC
        const C = 0b0000_0001;
        const Z = 0b0000_0010;
        const I = 0b0000_0100;
        const D = 0b0000_1000;
        const B = 0b0001_0000;
        const Default = 0b0010_0100;
        const V = 0b0100_0000;
        const N = 0b1000_0000;
    }
}

pub trait Cpu {
    fn cycle(&mut self, bus: &mut impl Bus);
    fn queue_microcode(
        &mut self,
        pre_bus: fn(&mut Self),
        bus_mode: BusMode,
        post_bus: fn(&mut Self),
    );
    fn read_pc(&mut self);
    fn push_operand(&mut self);
    fn instruction(&mut self);
    fn decode(&mut self);
    // fn read_pc_inc(&mut self) -> BusMode;
    // fn decode(&mut self) -> BusMode;
    // fn read_operand(&mut self) -> BusMode;
    // fn read_operand_execute(&mut self) -> BusMode;
}

pub mod addressing {
    use crate::BusMode;

    use super::Cpu;

    pub struct Branch;
    pub struct Read;
    pub struct ReadWrite;
    pub struct Write;

    pub trait IOMode {}

    impl IOMode for Branch {}
    impl IOMode for Read {}
    impl IOMode for ReadWrite {}
    impl IOMode for Write {}

    pub trait AddressingMode<Cpu, Mode: IOMode> {
        fn enqueue(cpu: &mut Cpu);
    }

    pub struct Absolute;

    impl<CPU: Cpu> AddressingMode<CPU, Branch> for Absolute {
        fn enqueue(cpu: &mut CPU) {
            cpu.queue_microcode(CPU::read_pc, BusMode::Read, CPU::push_operand);
            cpu.queue_microcode(CPU::read_pc, BusMode::Read, CPU::instruction);
            cpu.queue_microcode(CPU::read_pc, BusMode::Read, CPU::decode);
        }
    }

    impl<CPU> AddressingMode<CPU, Read> for Absolute {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    impl<CPU> AddressingMode<CPU, Write> for Absolute {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    pub struct Accumulator;

    impl<CPU> AddressingMode<CPU, Read> for Accumulator {
        fn enqueue(cpu: &mut CPU) {
            todo!()
        }
    }

    pub struct Immediate;

    impl<CPU: Cpu> AddressingMode<CPU, Read> for Immediate {
        fn enqueue(cpu: &mut CPU) {
            todo!()
            // cpu.queue_microcode(CPU::read_operand_execute);
            // cpu.queue_microcode(CPU::decode);
        }
    }
}
