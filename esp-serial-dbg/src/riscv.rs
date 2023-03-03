use super::with_serial;
use crate::hal::riscv;
use crate::hal::trapframe::TrapFrame;
use core::arch::asm;

pub fn init() {
    set_breakpoint(0, 0xffffffff); // no breakpoints without this?
}

pub fn set_breakpoint(id: u8, addr: u32) {
    unsafe {
        let tdata = (1 << 3) | (1 << 6) | (1 << 2);
        let tcontrol = 1 << 3;
        asm!(
            "
            csrw 0x7a0, {id} // tselect
            csrrs {tcontrol}, 0x7a5, {tcontrol} // tcontrol
            csrrs {tdata}, 0x7a1, {tdata} // tdata1
            csrw 0x7a2, {addr} // tdata2
        
        ", id = in(reg) id,
        addr = in(reg) addr,
        tdata = in(reg) tdata,
        tcontrol = in(reg) tcontrol,
        );
    }
}

pub fn clear_breakpoint(id: u8) {
    unsafe {
        let tdata = (1 << 3) | (1 << 6) | (1 << 2);
        let tcontrol = 1 << 3;
        asm!(
            "
            csrw 0x7a0, {id} // tselect
            csrrc {tcontrol}, 0x7a5, {tcontrol} // tcontrol
            csrrc {tdata}, 0x7a1, {tdata} // tdata1
        
        ", id = in(reg) id,
        tdata = in(reg) tdata,
        tcontrol = in(reg) tcontrol,
        );
    }
}

#[export_name = "ExceptionHandler"]
fn exception_handler(context: &mut TrapFrame) {
    let mepc = riscv::register::mepc::read();
    let code = riscv::register::mcause::read().code() & 0xff;
    let _mtval = riscv::register::mtval::read();

    if code == 3 {
        // breakpoint
        critical_section::with(|_cs| {
            with_serial(|serial| {
                let regs_raw = serialze_registers(context, mepc);

                crate::write_response(
                    serial,
                    crate::HIT_BREAKPOINT_RESPONSE,
                    regs_raw.len(),
                    &mut regs_raw.into_iter(),
                );

                // process commands while halted
                crate::serial_com_halted(serial, context, mepc);
            });
        });
    } else {
        // handle some other exception somehow?
        panic!("Exception! {:x}  {:x} {:?}", code, mepc, context);
    }
}

pub fn serialze_registers(context: &mut TrapFrame, mepc: usize) -> [u8; 32 * 4] {
    let mut regs_raw = [0u8; 32 * 4];
    regs_raw[0..][..4].copy_from_slice(&context.ra.to_le_bytes());
    regs_raw[4..][..4].copy_from_slice(&context.t0.to_le_bytes());
    regs_raw[8..][..4].copy_from_slice(&context.t1.to_le_bytes());
    regs_raw[12..][..4].copy_from_slice(&context.t2.to_le_bytes());
    regs_raw[16..][..4].copy_from_slice(&context.t3.to_le_bytes());
    regs_raw[20..][..4].copy_from_slice(&context.t4.to_le_bytes());
    regs_raw[24..][..4].copy_from_slice(&context.t5.to_le_bytes());
    regs_raw[28..][..4].copy_from_slice(&context.t6.to_le_bytes());
    regs_raw[32..][..4].copy_from_slice(&context.a0.to_le_bytes());
    regs_raw[36..][..4].copy_from_slice(&context.a1.to_le_bytes());
    regs_raw[40..][..4].copy_from_slice(&context.a2.to_le_bytes());
    regs_raw[44..][..4].copy_from_slice(&context.a3.to_le_bytes());
    regs_raw[48..][..4].copy_from_slice(&context.a4.to_le_bytes());
    regs_raw[52..][..4].copy_from_slice(&context.a5.to_le_bytes());
    regs_raw[56..][..4].copy_from_slice(&context.a6.to_le_bytes());
    regs_raw[60..][..4].copy_from_slice(&context.a7.to_le_bytes());
    regs_raw[64..][..4].copy_from_slice(&context.s0.to_le_bytes());
    regs_raw[68..][..4].copy_from_slice(&context.s1.to_le_bytes());
    regs_raw[72..][..4].copy_from_slice(&context.s2.to_le_bytes());
    regs_raw[76..][..4].copy_from_slice(&context.s3.to_le_bytes());
    regs_raw[80..][..4].copy_from_slice(&context.s4.to_le_bytes());
    regs_raw[84..][..4].copy_from_slice(&context.s5.to_le_bytes());
    regs_raw[88..][..4].copy_from_slice(&context.s6.to_le_bytes());
    regs_raw[92..][..4].copy_from_slice(&context.s7.to_le_bytes());
    regs_raw[96..][..4].copy_from_slice(&context.s8.to_le_bytes());
    regs_raw[100..][..4].copy_from_slice(&context.s9.to_le_bytes());
    regs_raw[104..][..4].copy_from_slice(&context.s10.to_le_bytes());
    regs_raw[108..][..4].copy_from_slice(&context.s11.to_le_bytes());
    regs_raw[112..][..4].copy_from_slice(&context.gp.to_le_bytes());
    regs_raw[116..][..4].copy_from_slice(&context.tp.to_le_bytes());
    regs_raw[120..][..4].copy_from_slice(&context.sp.to_le_bytes());
    regs_raw[124..][..4].copy_from_slice(&mepc.to_le_bytes());
    regs_raw
}
