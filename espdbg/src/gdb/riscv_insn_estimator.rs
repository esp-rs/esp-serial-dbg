use espdbg::RiscvRegisters;

pub fn riscv_insn_estimator(insn: [u8; 4], pc: u32, regs: RiscvRegisters) -> Vec<u32> {
    let mut candiates = Vec::new();

    let insn_len = if insn[0] & 0b11 == 0b11 { 4 } else { 2 };
    candiates.push(pc + insn_len);

    match insn_len {
        4 => {
            let inst = u32::from_le_bytes(insn);

            if (inst & 0b111_00000_11111_11) == 0b000_00000_11001_11 {
                // JALR / JR
                let offset = (inst & 0b111111111111_00000_000_00000_00000_00) >> 20;
                let rs1 = (inst & 0b000000000000_11111_000_00000_00000_00) >> 15;
                candiates
                    .push(((get_reg(regs, rs1 as u8) as i64 + sext12(offset) as i64) as u32) & !1);
            } else if (inst & 0b11111_11) == 0b11011_11 {
                // JAL
                let offset_20 = (inst & 0b100000000000_00000_000_00000_00000_00) >> 31;
                let offset_10_1 = (inst & 0b011111111110_00000_000_00000_00000_00) >> 21;
                let offset_19_12 = (inst & 0b000000000000_11111_111_00000_00000_00) >> 12;
                let offset_11 = (inst & 0b000000000001_00000_000_00000_00000_00) >> 20;
                let offset = (offset_10_1 << 1)
                    | (offset_11 << 11)
                    | (offset_19_12 << 12)
                    | (offset_20 << 20);

                candiates.push(((pc as i64 + sext20(offset) as i64) as u32) & !1);
            } else if (inst & 0b11111_11) == 0b11000_11 {
                // BEQ, BNE, BLT, GE, BLTU, BGEU
                let offset_12 = (inst & 0b100000000000_00000_000_00000_00000_00) >> 31;
                let offset_10_5 = (inst & 0b011111100000_00000_000_00000_00000_00) >> 25;
                let offset_4_1 = (inst & 0b1111_0_00000_00) >> 8;
                let offset_11 = (inst & 0b1_00000_00) >> 7;
                let offset =
                    (offset_12 << 12) | (offset_11 << 11) | (offset_10_5 << 5) | (offset_4_1 << 1);

                candiates.push(((pc as i64 + sext12(offset) as i64) as u32) & !1);
            }
        }
        2 => {
            let inst = u16::from_le_bytes(insn[0..2].try_into().unwrap());
            println!("inst = {:016b}", inst);

            if (inst & 0b111_00000000000_11) == 0b101_00000000000_01 {
                // C.J
                let imm = ((inst & 0b000_11111111111_00) as u32) >> 2;
                let offset_5 = imm & 0b1;
                let offset_3_1 = (imm & 0b1110) >> 1;
                let offset_7 = (imm & 0b10000) >> 4;
                let offset_6 = (imm & 0b100000) >> 5;
                let offset_10 = (imm & 0b1000000) >> 6;
                let offset_9_8 = (imm & 0b110000000) >> 7;
                let offset_4 = (imm & 0b1000000000) >> 9;
                let offset_11 = (imm & 0b10000000000) >> 10;

                let offset = (offset_3_1 << 1)
                    | (offset_4 << 4)
                    | (offset_5 << 5)
                    | (offset_6 << 6)
                    | (offset_7 << 7)
                    | (offset_9_8 << 8)
                    | (offset_10 << 10)
                    | (offset_11 << 11);

                candiates.push(((pc as i64 + sext11(offset) as i64) as u32) & !1);
            } else if (inst & 0b111_1_00000_11111_11) == 0b100_00000000000_10 {
                // C.JR
                let reg = ((inst & 0b11111_00000_00) as u32) >> 7;
                candiates.push(get_reg(regs, reg as u8));
            }

            // C.JAL
            // C.J
            // C.BEQZ
            // C.BNEZ
            // C.JALR
        }
        _ => panic!("Unexpected insn_len"),
    }

    candiates
}

fn sext11(value: u32) -> i32 {
    if value & (1 << 10) != 0 {
        (0b100_0000_0000 - (value & 0b11_1111_1111) as i32) * -1
    } else {
        value as i32
    }
}

fn sext12(value: u32) -> i32 {
    if value & (1 << 11) != 0 {
        (0b1000_0000_0000 - (value & 0b111_1111_1111) as i32) * -1
    } else {
        value as i32
    }
}

fn sext20(value: u32) -> i32 {
    if value & (1 << 21) != 0 {
        (0b1000_0000_0000_0000_0000 - (value & 0b0111_1111_1111_1111_1111) as i32) * -1
    } else {
        value as i32
    }
}

fn get_reg(regs: RiscvRegisters, num: u8) -> u32 {
    match num {
        0 => 0,
        1 => regs.ra,
        2 => regs.sp,
        3 => regs.gp,
        4 => regs.tp,
        5 => regs.t0,
        6 => regs.t1,
        7 => regs.t2,
        8 => regs.s0,
        9 => regs.s1,
        10 => regs.a0,
        11 => regs.a1,
        12 => regs.a2,
        13 => regs.a3,
        14 => regs.a4,
        15 => regs.a5,
        16 => regs.a6,
        17 => regs.a7,
        18 => regs.s2,
        19 => regs.s3,
        20 => regs.s4,
        21 => regs.s5,
        22 => regs.s6,
        23 => regs.s7,
        24 => regs.s8,
        25 => regs.s9,
        26 => regs.s10,
        27 => regs.s11,
        28 => regs.t3,
        29 => regs.t4,
        30 => regs.t5,
        31 => regs.t6,
        _ => panic!("invalid register number"),
    }
}

#[test]
fn test_non_branching_uncompressed() {
    let pc = 0x42000070;
    let regs = RiscvRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x97, 0x11, 0xc8, 0xfd];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0], 0x42000074);
}

#[test]
fn test_non_branching_compressed() {
    let pc = 0x42000060;
    let regs = RiscvRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x01, 0x4c, 0xff, 0xff];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0], 0x42000062);
}

#[test]
fn test_branching_uncompressed_jr() {
    let pc = 0x42000024;
    let regs = RiscvRegisters {
        pc,
        ra: 0x42008_000,
        ..Default::default()
    };
    let isn = [0x67, 0x80, 0x80, 0xca];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x42000028);
    assert_eq!(res[1], 0x42007ca8);
}

#[test]
fn test_branching_uncompressed_jal() {
    let pc = 0x42000308;
    let regs = RiscvRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0xef, 0x00, 0xc0, 0x16];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x4200030c);
    assert_eq!(res[1], 0x42000474);
}

#[test]
fn test_branching_uncompressed_beq() {
    let pc = 0x42000b74;
    let regs = RiscvRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x63, 0x05, 0xb5, 0x00];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x42000b78);
    assert_eq!(res[1], 0x42000b7e);
}

#[test]
fn test_branching_uncompressed_bne() {
    let pc = 0x420000cc;
    let regs = RiscvRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x63, 0x18, 0xb5, 0x00];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x420000d0);
    assert_eq!(res[1], 0x420000dc);
}

#[test]
fn test_branching_uncompressed_blt() {
    let pc = 0x4200125e;
    let regs = RiscvRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x63, 0x44, 0xb5, 0x00];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x42001262);
    assert_eq!(res[1], 0x42001266);
}

#[test]
fn test_branching_compressed_j() {
    let pc = 0x42002322;
    let regs = RiscvRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x61, 0xbf, 0x00, 0x00];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x42002324);
    assert_eq!(res[1], 0x420022ba);
}

#[test]
fn test_branching_compressed_jr() {
    let pc = 0x4200a992;
    let regs = RiscvRegisters {
        pc,
        a0: 0x42000000,
        ..Default::default()
    };
    let isn = [0x02, 0x85, 0x00, 0x00];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x4200a994);
    assert_eq!(res[1], 0x42000000);
}

#[test]
fn test_non_branching_compressed_sw() {
    let pc = 0x420022c8;
    let regs = RiscvRegisters {
        pc,
        a0: 0x42000000,
        ..Default::default()
    };
    let isn = [0x85, 0x45, 0x00, 0x00];

    let res = riscv_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0], 0x420022ca);
}
