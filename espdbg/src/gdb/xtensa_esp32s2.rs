use gdbstub::arch::Registers;

use super::EspRegisters;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct XtensaEsp32S2Registers {
    pub regs: crate::XtensaRegisters,
}

impl Registers for XtensaEsp32S2Registers {
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

        // see https://github.com/espressif/xtensa-overlays/blob/dd1cf19f6eb327a9db51043439974a6de13f5c7f/xtensa_esp32s2/gdb/gdb/regformats/reg-xtensa.dat
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
        write_u32(&mut write_byte, self.regs.sar);
        write_u32(&mut write_byte, 0); // windowbase
        write_u32(&mut write_byte, 0); // windowstart
        write_u32(&mut write_byte, 0); // configid0
        write_u32(&mut write_byte, 0); // configid1
        write_u32(&mut write_byte, self.regs.ps);
        write_u32(&mut write_byte, self.regs.threadptr);
        write_u32(&mut write_byte, 0); // gpio_out

        write_u32(&mut write_byte, 0);
    }

    fn gdb_deserialize(&mut self, _bytes: &[u8]) -> Result<(), ()> {
        println!("TODO gdb_deserialize");
        Ok(())
    }
}

impl EspRegisters for XtensaEsp32S2Registers {
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
        // TODO
        r#"<?xml version="1.0"?>
<!DOCTYPE memory-map
    PUBLIC "+//IDN gnu.org//DTD GDB Memory Map V1.0//EN"
            "http://sourceware.org/gdb/gdb-memory-map.dtd">
<memory-map>
    <memory type="rom" start="0x40080000" length="0x780000"/>
    <memory type="rom" start="0x3F000000" length="0xF80000"/>
    <memory type="ram" start="0x40020000" length="0x50000"/>
    <memory type="ram" start="0x40070000" length="0x2000"/>
    <memory type="ram" start="0x3ff9e000" length="0x2000"/>
    <memory type="ram" start="0x3FFB0000" length="0x50000"/>
</memory-map>"#
    }

    fn sw_breakpoint_opcode() -> (usize, [u8; 3]) {
        (2, [0x2d, 0xf1, 0x00])
    }

    fn hw_breakpoint_start() -> u8 {
        0
    }

    fn hw_breakpoint_end() -> u8 {
        1
    }
}
