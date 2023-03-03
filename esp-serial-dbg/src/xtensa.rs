use super::with_serial;
use crate::hal::trapframe::TrapFrame;
use core::arch::asm;

pub fn init() {
    // nothing
}

pub fn set_breakpoint(id: u8, addr: u32) {
    /*
    #define IBREAKA_0	128
    #define IBREAKA_1	129
    #define IBREAKENABLE	96
    */

    let mut _breakenable: u32 = 0;

    if id == 0 {
        unsafe {
            asm!(
                "
                wsr {addr}, 128
                rsr {breakenable}, 96
                ",
                addr = in(reg) addr,
                breakenable = out(reg) _breakenable,
            );
            _breakenable |= 0b01;
            asm!(
                "
                wsr {breakenable}, 96
                ",
                breakenable = in(reg) _breakenable,
            );
        }
    } else {
        unsafe {
            asm!(
                "
                wsr {addr}, 129
                rsr {breakenable}, 96
                ",
                addr = in(reg) addr,
                breakenable = out(reg) _breakenable,
            );
            _breakenable |= 0b10;
            asm!(
                "
                wsr {breakenable}, 96
                ",
                breakenable = in(reg) _breakenable,
            );
        }
    }
}

pub fn clear_breakpoint(id: u8) {
    /*
    #define IBREAKA_0	128
    #define IBREAKA_1	129
    #define IBREAKENABLE	96
    */

    let addr = 0;
    let mut _breakenable: u32 = 0;

    if id == 0 {
        unsafe {
            asm!(
                "
                wsr {addr}, 128
                rsr {breakenable}, 96
                ",
                addr = in(reg) addr,
                breakenable = out(reg) _breakenable,
            );
            _breakenable &= !0b01;
            asm!(
                "
                wsr {breakenable}, 96
                ",
                breakenable = in(reg) _breakenable,
            );
        }
    } else {
        unsafe {
            asm!(
                "
                wsr {addr}, 129
                rsr {breakenable}, 96
                ",
                addr = in(reg) addr,
                breakenable = out(reg) _breakenable,
            );
            _breakenable &= !0b10;
            asm!(
                "
                wsr {breakenable}, 96
                ",
                breakenable = in(reg) _breakenable,
            );
        }
    }
}

#[no_mangle]
fn level6_interrupt(save_frame: &mut TrapFrame) {
    with_serial(|serial| {
        let regs_raw = serialze_registers(save_frame, save_frame.PC as usize);

        crate::write_response(
            serial,
            crate::HIT_BREAKPOINT_RESPONSE,
            regs_raw.len(),
            &mut regs_raw.into_iter(),
        );

        // process commands while halted
        crate::serial_com_halted(serial, save_frame, save_frame.PC as usize);
    });
}

pub fn serialze_registers(context: &mut TrapFrame, _mepc: usize) -> [u8; 54 * 4] {
    let mut regs_raw = [0u8; 54 * 4];
    regs_raw[0..][..4].copy_from_slice(&context.PC.to_le_bytes());
    regs_raw[4..][..4].copy_from_slice(&context.PS.to_le_bytes());
    regs_raw[8..][..4].copy_from_slice(&context.A0.to_le_bytes());
    regs_raw[12..][..4].copy_from_slice(&context.A1.to_le_bytes());
    regs_raw[16..][..4].copy_from_slice(&context.A2.to_le_bytes());
    regs_raw[20..][..4].copy_from_slice(&context.A3.to_le_bytes());
    regs_raw[24..][..4].copy_from_slice(&context.A4.to_le_bytes());
    regs_raw[28..][..4].copy_from_slice(&context.A5.to_le_bytes());
    regs_raw[32..][..4].copy_from_slice(&context.A6.to_le_bytes());
    regs_raw[36..][..4].copy_from_slice(&context.A7.to_le_bytes());
    regs_raw[40..][..4].copy_from_slice(&context.A8.to_le_bytes());
    regs_raw[44..][..4].copy_from_slice(&context.A9.to_le_bytes());
    regs_raw[48..][..4].copy_from_slice(&context.A10.to_le_bytes());
    regs_raw[52..][..4].copy_from_slice(&context.A11.to_le_bytes());
    regs_raw[56..][..4].copy_from_slice(&context.A12.to_le_bytes());
    regs_raw[60..][..4].copy_from_slice(&context.A13.to_le_bytes());
    regs_raw[64..][..4].copy_from_slice(&context.A14.to_le_bytes());
    regs_raw[68..][..4].copy_from_slice(&context.A15.to_le_bytes());
    regs_raw[72..][..4].copy_from_slice(&context.SAR.to_le_bytes());
    regs_raw[76..][..4].copy_from_slice(&context.EXCCAUSE.to_le_bytes());
    regs_raw[80..][..4].copy_from_slice(&context.EXCVADDR.to_le_bytes());
    regs_raw[84..][..4].copy_from_slice(&context.LBEG.to_le_bytes());
    regs_raw[88..][..4].copy_from_slice(&context.LEND.to_le_bytes());
    regs_raw[92..][..4].copy_from_slice(&context.LCOUNT.to_le_bytes());
    regs_raw[96..][..4].copy_from_slice(&context.THREADPTR.to_le_bytes());
    regs_raw[100..][..4].copy_from_slice(&context.SCOMPARE1.to_le_bytes());
    regs_raw[104..][..4].copy_from_slice(&context.BR.to_le_bytes());
    regs_raw[108..][..4].copy_from_slice(&context.ACCLO.to_le_bytes());
    regs_raw[112..][..4].copy_from_slice(&context.ACCHI.to_le_bytes());
    regs_raw[116..][..4].copy_from_slice(&context.M0.to_le_bytes());
    regs_raw[120..][..4].copy_from_slice(&context.M1.to_le_bytes());
    regs_raw[124..][..4].copy_from_slice(&context.M2.to_le_bytes());
    regs_raw[128..][..4].copy_from_slice(&context.M3.to_le_bytes());
    regs_raw[132..][..4].copy_from_slice(&context.F64R_LO.to_le_bytes());
    regs_raw[136..][..4].copy_from_slice(&context.F64R_HI.to_le_bytes());
    regs_raw[140..][..4].copy_from_slice(&context.F64S.to_le_bytes());
    regs_raw[144..][..4].copy_from_slice(&context.FCR.to_le_bytes());
    regs_raw[148..][..4].copy_from_slice(&context.FSR.to_le_bytes());
    regs_raw[150..][..4].copy_from_slice(&context.F0.to_le_bytes());
    regs_raw[154..][..4].copy_from_slice(&context.F1.to_le_bytes());
    regs_raw[160..][..4].copy_from_slice(&context.F2.to_le_bytes());
    regs_raw[164..][..4].copy_from_slice(&context.F3.to_le_bytes());
    regs_raw[168..][..4].copy_from_slice(&context.F4.to_le_bytes());
    regs_raw[172..][..4].copy_from_slice(&context.F5.to_le_bytes());
    regs_raw[176..][..4].copy_from_slice(&context.F6.to_le_bytes());
    regs_raw[180..][..4].copy_from_slice(&context.F7.to_le_bytes());
    regs_raw[184..][..4].copy_from_slice(&context.F8.to_le_bytes());
    regs_raw[188..][..4].copy_from_slice(&context.F9.to_le_bytes());
    regs_raw[192..][..4].copy_from_slice(&context.F10.to_le_bytes());
    regs_raw[196..][..4].copy_from_slice(&context.F11.to_le_bytes());
    regs_raw[200..][..4].copy_from_slice(&context.F12.to_le_bytes());
    regs_raw[204..][..4].copy_from_slice(&context.F13.to_le_bytes());
    regs_raw[208..][..4].copy_from_slice(&context.F14.to_le_bytes());
    regs_raw[212..][..4].copy_from_slice(&context.F15.to_le_bytes());
    regs_raw
}
