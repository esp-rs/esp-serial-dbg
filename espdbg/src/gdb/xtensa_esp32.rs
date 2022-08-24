use gdbstub::arch::Registers;

use super::EspRegisters;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct XtensaEsp32Registers {
    pub regs: crate::XtensaRegisters,
}

impl Registers for XtensaEsp32Registers {
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

        // see https://github.com/espressif/xtensa-overlays/blob/dd1cf19f6eb327a9db51043439974a6de13f5c7f/xtensa_esp32/gdb/gdb/regformats/reg-xtensa.dat

        write_u32(&mut write_byte, self.regs.pc);
        write_u32(&mut write_byte, self.regs.a0);
        write_u32(&mut write_byte, self.regs.a1);
        write_u32(&mut write_byte, self.regs.a2);
        write_u32(&mut write_byte, self.regs.a3);
        write_u32(&mut write_byte, self.regs.a4);
        write_u32(&mut write_byte, self.regs.a5);
        write_u32(&mut write_byte, self.regs.a6);
        write_u32(&mut write_byte, self.regs.a7);
        write_u32(&mut write_byte, self.regs.a8);
        write_u32(&mut write_byte, self.regs.a9);
        write_u32(&mut write_byte, self.regs.a10);
        write_u32(&mut write_byte, self.regs.a11);
        write_u32(&mut write_byte, self.regs.a12);
        write_u32(&mut write_byte, self.regs.a13);
        write_u32(&mut write_byte, self.regs.a14);
        write_u32(&mut write_byte, self.regs.a15);
        for i in 0..48 {
            write_u32(&mut write_byte, (i) as u32);
        }
        write_u32(&mut write_byte, self.regs.lbeg);
        write_u32(&mut write_byte, self.regs.lend);
        write_u32(&mut write_byte, self.regs.lcount);
        write_u32(&mut write_byte, self.regs.sar);
        write_u32(&mut write_byte, 0); // windowbase
        write_u32(&mut write_byte, 0); // windowstart
        write_u32(&mut write_byte, 0); // configid0
        write_u32(&mut write_byte, 0); // configid1
        write_u32(&mut write_byte, self.regs.ps);
        write_u32(&mut write_byte, self.regs.threadptr);
        write_u32(&mut write_byte, 0); // br
        write_u32(&mut write_byte, self.regs.scompare1);
        write_u32(&mut write_byte, self.regs.acclo);
        write_u32(&mut write_byte, self.regs.acchi);
        write_u32(&mut write_byte, self.regs.m0);
        write_u32(&mut write_byte, self.regs.m1);
        write_u32(&mut write_byte, self.regs.m2);
        write_u32(&mut write_byte, self.regs.m3);
        write_u32(&mut write_byte, 0); // expstate
        write_u32(&mut write_byte, self.regs.f64r_lo);
        write_u32(&mut write_byte, self.regs.f64r_hi);
        write_u32(&mut write_byte, self.regs.f64s);
        write_u32(&mut write_byte, self.regs.f0);
        write_u32(&mut write_byte, self.regs.f1);
        write_u32(&mut write_byte, self.regs.f2);
        write_u32(&mut write_byte, self.regs.f3);
        write_u32(&mut write_byte, self.regs.f4);
        write_u32(&mut write_byte, self.regs.f5);
        write_u32(&mut write_byte, self.regs.f6);
        write_u32(&mut write_byte, self.regs.f7);
        write_u32(&mut write_byte, self.regs.f8);
        write_u32(&mut write_byte, self.regs.f9);
        write_u32(&mut write_byte, self.regs.f10);
        write_u32(&mut write_byte, self.regs.f11);
        write_u32(&mut write_byte, self.regs.f12);
        write_u32(&mut write_byte, self.regs.f13);
        write_u32(&mut write_byte, self.regs.f14);
        write_u32(&mut write_byte, self.regs.f15);
        write_u32(&mut write_byte, self.regs.fcr);
        write_u32(&mut write_byte, self.regs.fsr);
    }

    fn gdb_deserialize(&mut self, _bytes: &[u8]) -> Result<(), ()> {
        println!("TODO gdb_deserialize");
        Ok(())
    }
}

impl EspRegisters for XtensaEsp32Registers {
    fn set_regs(&mut self, regs: crate::Registers) {
        match regs {
            espdbg::Registers::Xtensa(regs) => {
                self.regs = regs;
            }
            espdbg::Registers::Riscv(_) => panic!("You mixed up Xtensa and RiscV"),
        }
    }

    fn architecture() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>xtensa</architecture></target>"#)
    }

    fn memory_map() -> &'static str {
        r#"<?xml version="1.0"?>
<!DOCTYPE memory-map
    PUBLIC "+//IDN gnu.org//DTD GDB Memory Map V1.0//EN"
            "http://sourceware.org/gdb/gdb-memory-map.dtd">
<memory-map>
    <memory type="rom" start="0x400D0000" length="0x330000"/>
    <memory type="rom" start="0x3F400000" length="0x330000"/>
    <memory type="ram" start="0x40070000" length="0x60000"/>
    <memory type="ram" start="0x3FFAE000" length="0x52000"/>
    <memory type="ram" start="0x3FF80000" length="0x2000"/>
    <memory type="ram" start="0x50000000" length="0x2000"/>
</memory-map>"#
    }

    fn sw_breakpoint_opcode() -> (usize, [u8; 3]) {
        (3, [0xf0, 0x41, 0x00])
    }

    fn hw_breakpoint_start() -> u8 {
        0
    }

    fn hw_breakpoint_end() -> u8 {
        1
    }
}
