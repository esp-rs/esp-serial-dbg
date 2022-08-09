use gdbstub::arch::Registers;

use super::EspRegisters;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct RiscvRegisters {
    pub regs: crate::RiscvRegisters,
}

impl Registers for RiscvRegisters {
    type ProgramCounter = u32;

    fn pc(&self) -> Self::ProgramCounter {
        self.regs.pc
    }

    fn gdb_serialize(&self, mut write_byte: impl FnMut(Option<u8>)) {
        fn write_u32(write_byte: &mut impl FnMut(Option<u8>), word: u32) {
            let le_bytes = word.to_le_bytes();
            write_byte(Some(le_bytes[0]));
            write_byte(Some(le_bytes[1]));
            write_byte(Some(le_bytes[2]));
            write_byte(Some(le_bytes[3]));
        }

        // see https://github.com/bminor/binutils-gdb/blob/master/gdb/features/riscv/32bit-cpu.xml
        write_u32(&mut write_byte, 0);
        write_u32(&mut write_byte, self.regs.ra);
        write_u32(&mut write_byte, self.regs.sp);
        write_u32(&mut write_byte, self.regs.gp);
        write_u32(&mut write_byte, self.regs.tp);
        write_u32(&mut write_byte, self.regs.t0);
        write_u32(&mut write_byte, self.regs.t1);
        write_u32(&mut write_byte, self.regs.t2);
        write_u32(&mut write_byte, self.regs.s0);
        write_u32(&mut write_byte, self.regs.s1);
        write_u32(&mut write_byte, self.regs.a0);
        write_u32(&mut write_byte, self.regs.a1);
        write_u32(&mut write_byte, self.regs.a2);
        write_u32(&mut write_byte, self.regs.a3);
        write_u32(&mut write_byte, self.regs.a4);
        write_u32(&mut write_byte, self.regs.a5);
        write_u32(&mut write_byte, self.regs.a6);
        write_u32(&mut write_byte, self.regs.a7);
        write_u32(&mut write_byte, self.regs.s2);
        write_u32(&mut write_byte, self.regs.s3);
        write_u32(&mut write_byte, self.regs.s4);
        write_u32(&mut write_byte, self.regs.s5);
        write_u32(&mut write_byte, self.regs.s6);
        write_u32(&mut write_byte, self.regs.s7);
        write_u32(&mut write_byte, self.regs.s8);
        write_u32(&mut write_byte, self.regs.s9);
        write_u32(&mut write_byte, self.regs.s10);
        write_u32(&mut write_byte, self.regs.s11);
        write_u32(&mut write_byte, self.regs.t3);
        write_u32(&mut write_byte, self.regs.t4);
        write_u32(&mut write_byte, self.regs.t5);
        write_u32(&mut write_byte, self.regs.t6);
        write_u32(&mut write_byte, self.regs.pc);
    }

    fn gdb_deserialize(&mut self, _bytes: &[u8]) -> Result<(), ()> {
        println!("TODO gdb_deserialize");
        Ok(())
    }
}

impl EspRegisters for RiscvRegisters {
    fn set_regs(&mut self, regs: crate::Registers) {
        match regs {
            espdbg::Registers::Riscv(regs) => {
                self.regs = regs;
            }
            espdbg::Registers::Xtensa(_) => panic!("You mixed up Xtensa and RiscV"),
        }
    }

    fn architecture() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>riscv:rv32</architecture></target>"#)
    }

    fn memory_map() -> &'static str {
        r#"<?xml version="1.0"?>
<!DOCTYPE memory-map
    PUBLIC "+//IDN gnu.org//DTD GDB Memory Map V1.0//EN"
            "http://sourceware.org/gdb/gdb-memory-map.dtd">
<memory-map>
    <memory type="ram" start="0x3FC80000" length="0x60000"/>
    <memory type="rom" start="0x3C000000" length="0x800000"/>
    <memory type="rom" start="0x3FF00000" length="0x20000"/>
    <memory type="rom" start="0x40000000" length="0x60000"/>
    <memory type="ram" start="0x4037C000" length="0x64000"/>
    <memory type="ram" start="0x50000000" length="0x2000"/>
    <memory type="rom" start="0x42000000" length="0x800000"/>
    <memory type="ram" start="0x600FE000" length="0x2000"/>
</memory-map>"#
    }

    fn sw_breakpoint_opcode() -> (usize, [u8; 3]) {
        (2, [0x02, 0x90, 0x00])
    }

    fn hw_breakpoint_start() -> u8 {
        1
    }

    fn hw_breakpoint_end() -> u8 {
        7
    }
}
