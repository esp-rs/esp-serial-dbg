use espdbg::XtensaRegisters;

const LENGTH_TABLE: [u8; 256] = [
    3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4,
    3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4,
    3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4,
    3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4,
    3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4,
    3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4,
    3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4,
    3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 4, 4,
];

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum InsnFormat {
    RRI8 {
        op0: u8,
        t: u8,
        s: u8,
        r: u8,
        imm: u8,
    },
    CALL {
        op0: u8,
        n: u8,
        offset: u32,
    },
    CALLX {
        op0: u8,
        n: u8,
        m: u8,
        s: u8,
        r: u8,
        op1: u8,
        op2: u8,
    },
    BRI8 {
        op0: u8,
        n: u8,
        m: u8,
        s: u8,
        r: u8,
        imm: u8,
    },
    BRI12 {
        op0: u8,
        n: u8,
        m: u8,
        s: u8,
        imm: u16,
    },
    RI6 {
        op0: u8,
        z: bool,
        i: bool,
        s: u8,
        imm: u8,
    },
}

impl InsnFormat {
    fn rri8(insn: [u8; 4]) -> InsnFormat {
        let op0 = insn[0] & 0b1111;
        let t = (insn[0] & 0b11110000) >> 4;
        let s = insn[1] & 0b1111;
        let r = (insn[1] & 0b11110000) >> 4;
        let imm = insn[2];
        InsnFormat::RRI8 { op0, t, s, r, imm }
    }

    fn bri12(insn: [u8; 4]) -> InsnFormat {
        let op0 = insn[0] & 0b1111;
        let n = (insn[0] & 0b00110000) >> 4;
        let m = (insn[0] & 0b11000000) >> 6;
        let s = insn[1] & 0b1111;
        let imm = (insn[2] as u16) << 4 | ((insn[1] & 0b11110000) >> 4) as u16;
        InsnFormat::BRI12 { op0, n, m, s, imm }
    }

    fn bri8(insn: [u8; 4]) -> InsnFormat {
        let op0 = insn[0] & 0b1111;
        let n = (insn[0] & 0b00110000) >> 4;
        let m = (insn[0] & 0b11000000) >> 6;
        let s = insn[1] & 0b1111;
        let r = (insn[1] & 0b11110000) >> 4;
        let imm = insn[2];
        InsnFormat::BRI8 {
            op0,
            n,
            m,
            s,
            r,
            imm,
        }
    }

    fn ri6(insn: [u8; 4]) -> InsnFormat {
        let op0 = insn[0] & 0b1111;
        let imm = (insn[0] & 0b110000) >> 4;
        let z = (insn[0] & 0b1000000) != 0;
        let i = (insn[0] & 0b10000000) != 0;
        let s = insn[1] & 0b1111;
        let imm = (insn[1] & 0b11110000) >> 4 | imm << 4;
        InsnFormat::RI6 { op0, z, i, s, imm }
    }

    fn call(insn: [u8; 4]) -> InsnFormat {
        let op0 = insn[0] & 0b1111;
        let n = (insn[0] & 0b110000) >> 4;
        let offset =
            ((insn[0] & 0b1100_0000) >> 6) as u32 | (insn[1] as u32) << 2 | (insn[2] as u32) << 10;
        InsnFormat::CALL { op0, n, offset }
    }

    fn callx(insn: [u8; 4]) -> InsnFormat {
        let op0 = insn[0] & 0b1111;
        let n = (insn[0] & 0b110000) >> 4;
        let m = (insn[0] & 0b11000000) >> 6;
        let s = insn[1] & 0b1111;
        let r = (insn[1] & 0b11110000) >> 4;
        let op1 = insn[2] & 0b1111;
        let op2 = (insn[2] & 0b11110000) >> 4;
        InsnFormat::CALLX {
            op0,
            n,
            m,
            s,
            r,
            op1,
            op2,
        }
    }
}

pub fn xtensa_insn_estimator(insn: [u8; 4], pc: u32, regs: XtensaRegisters) -> Vec<u32> {
    let mut candiates = Vec::new();

    let insn_size = LENGTH_TABLE[insn[0] as usize] as u32;
    candiates.push(pc + insn_size);

    let op0 = insn[0] & 0b1111;
    if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b0100 {
        // BALL
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b1000 {
        // BANY
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b0101 {
        // BBC
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11100000) >> 4 == 0b0110 {
        // BBCI
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b1101 {
        // BBS
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11100000) >> 4 == 0b1110 {
        // BBSI
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b0001 {
        // BEQ
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b00100111 {
        // BEQI
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b00010111 {
        // BEQZ
        if let InsnFormat::BRI12 { imm, .. } = InsnFormat::bri12(insn) {
            candiates.push((pc as i64 + signed12(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] & 0b11001111 == 0b10001100 {
        // BEQZ.N
        if let InsnFormat::RI6 { imm, .. } = InsnFormat::ri6(insn) {
            candiates.push((pc as i64 + signed6(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b1010 {
        // BGE
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b11100110 {
        // BGEI
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b1011 {
        // BGEU
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b11100110 {
        // BGEUI
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::bri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b11010110 {
        // BGEZ
        if let InsnFormat::BRI12 { imm, .. } = InsnFormat::bri12(insn) {
            candiates.push((pc as i64 + signed12(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b0010 {
        // BLT
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b10100110 {
        // BLTI
        if let InsnFormat::BRI8 { imm, .. } = InsnFormat::bri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b0011 {
        // BLTU
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b10110110 {
        // BLTUI
        if let InsnFormat::BRI8 { imm, .. } = InsnFormat::bri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b10010110 {
        // BLTZ
        if let InsnFormat::BRI12 { imm, .. } = InsnFormat::bri12(insn) {
            candiates.push((pc as i64 + signed12(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b1100 {
        // BNALL
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b1001 {
        // BNE
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b01100110 {
        // BNEI
        if let InsnFormat::BRI8 { imm, .. } = InsnFormat::bri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] == 0b01010110 {
        // BNEZ
        if let InsnFormat::BRI12 { imm, .. } = InsnFormat::bri12(insn) {
            candiates.push((pc as i64 + signed12(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] & 0b11001111 == 0b11001100 {
        // BNEZ.N
        if let InsnFormat::RI6 { imm, .. } = InsnFormat::ri6(insn) {
            candiates.push((pc as i64 + signed6(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0111 && (insn[1] & 0b11110000) >> 4 == 0b0111 {
        // BNONE
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if op0 == 0b0110 && (insn[1] & 0b11110000) >> 4 == 0b0001 {
        // BT
        if let InsnFormat::RRI8 { imm, .. } = InsnFormat::rri8(insn) {
            candiates.push((pc as i64 + signed8(imm as u32) as i64 + 4) as u32);
        }
    } else if insn[0] & 0b111111 == 0b000101 {
        // CALL0
        if let InsnFormat::CALL { offset, .. } = InsnFormat::call(insn) {
            candiates.push(((pc & !0b11) as i64 + ((signed18(offset) + 1) as i64 * 4)) as u32);
        }
    } else if insn[0] & 0b111111 == 0b010101 {
        // CALL4
        if let InsnFormat::CALL { offset, .. } = InsnFormat::call(insn) {
            candiates.push(((pc & !0b11) as i64 + ((signed18(offset) + 1) as i64 * 4)) as u32);
        }
    } else if insn[0] & 0b111111 == 0b100101 {
        // CALL8
        if let InsnFormat::CALL { offset, .. } = InsnFormat::call(insn) {
            candiates.push(((pc & !0b11) as i64 + ((signed18(offset) + 1) as i64 * 4)) as u32);
        }
    } else if insn[0] & 0b111111 == 0b110101 {
        // CALL12
        if let InsnFormat::CALL { offset, .. } = InsnFormat::call(insn) {
            candiates.push(((pc & !0b11) as i64 + ((signed18(offset) + 1) as i64 * 4)) as u32);
        }
    } else if insn[0] == 0b11000000 && insn[1] & 0b11110000 == 0 && insn[2] == 0 {
        // CALLX0
        if let InsnFormat::CALLX { s, .. } = InsnFormat::callx(insn) {
            candiates.push(get_reg(regs, s));
        }
    } else if insn[0] == 0b11010000 && insn[1] & 0b11110000 == 0 && insn[2] == 0 {
        // CALLX4
        if let InsnFormat::CALLX { s, .. } = InsnFormat::callx(insn) {
            candiates.push(get_reg(regs, s));
        }
    } else if insn[0] == 0b11100000 && insn[1] & 0b11110000 == 0 && insn[2] == 0 {
        // CALLX8
        if let InsnFormat::CALLX { s, .. } = InsnFormat::callx(insn) {
            candiates.push(get_reg(regs, s));
        }
    } else if insn[0] == 0b11110000 && insn[1] & 0b11110000 == 0 && insn[2] == 0 {
        // CALLX12
        if let InsnFormat::CALLX { s, .. } = InsnFormat::callx(insn) {
            candiates.push(get_reg(regs, s));
        }
    } else if insn[0] & 0b111111 == 0b000110 {
        // J
        if let InsnFormat::CALL { offset, .. } = InsnFormat::call(insn) {
            candiates.push((pc as i64 + ((signed18(offset) + 4) as i64)) as u32);
        }
    } else if insn[0] == 0b10100000 && insn[1] & 0b11110000 == 0 && insn[2] == 0 {
        // JX
        if let InsnFormat::CALLX { s, .. } = InsnFormat::callx(insn) {
            candiates.push(get_reg(regs, s));
        }
    } else if insn[0] == 0b10000000 && insn[1] == 0 && insn[2] == 0 {
        // RET
        candiates.push(get_reg(regs, 0));
    } else if insn[0] == 0b10000000 && insn[1] == 0b11110000 {
        // RET.N
        candiates.push(get_reg(regs, 0));
    } else if insn[0] == 0b10000000 && insn[1] == 0 && insn[2] == 0 {
        // RETW
        candiates.push(get_reg(regs, 0)); // ???
    } else if insn[0] == 0b00011101 && insn[1] == 0b11110000 {
        // RETW.N
        candiates.push(get_reg(regs, 0));
    }

    // TODO LOOP - not that easy - apparently Rust doesn't generate those LOOP instructions for now?
    // TODO LOOPGTZ - not that easy - apparently Rust doesn't generate those LOOP instructions for now?
    // TODO LOOPNEZ - not that easy - apparently Rust doesn't generate those LOOP instructions for now?

    // some more RETURN operations

    candiates
}

fn signed18(value: u32) -> i32 {
    if value & (1 << 17) != 0 {
        (0b10_0000_0000_0000_0000 - (value & 0b1_1111_1111_1111_1111) as i32) * -1
    } else {
        value as i32
    }
}

fn signed8(value: u32) -> i32 {
    if value & (1 << 7) != 0 {
        (0b1000_0000 - (value & 0b0111_1111) as i32) * -1
    } else {
        value as i32
    }
}

fn signed6(value: u32) -> i32 {
    if value & (1 << 5) != 0 {
        (0b10_0000 - (value & 0b01_1111) as i32) * -1
    } else {
        value as i32
    }
}

fn signed12(value: u32) -> i32 {
    if value & (1 << 11) != 0 {
        (0b1000_0000_0000 - (value & 0b1111_1111_1111) as i32) * -1
    } else {
        value as i32
    }
}

fn get_reg(regs: XtensaRegisters, s: u8) -> u32 {
    match s {
        0 => regs.a0,
        1 => regs.a1,
        2 => regs.a2,
        3 => regs.a3,
        4 => regs.a4,
        5 => regs.a5,
        6 => regs.a6,
        7 => regs.a7,
        8 => regs.a8,
        9 => regs.a9,
        10 => regs.a10,
        11 => regs.a11,
        12 => regs.a12,
        13 => regs.a13,
        14 => regs.a14,
        15 => regs.a15,
        _ => panic!("unexpected register"),
    }
}

#[test]
fn test_non_branching1() {
    let pc = 0x420000fe;
    let regs = XtensaRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x01, 0x60, 0x6c, 0xff];

    let res = xtensa_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0], 0x42000101);
}

#[test]
fn test_non_branching2() {
    let pc = 0x42000101;
    let regs = XtensaRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x28, 0x00, 0xff, 0xff];

    let res = xtensa_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0], 0x42000103);
}

#[test]
fn test_branching_ball() {
    let pc = 0x4200032e;
    let regs = XtensaRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x37, 0x40, 0x70, 0xff];

    let res = xtensa_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x42000331);
    assert_eq!(res[1], 0x420003a2);
}

#[test]
fn test_branching_beqz_n() {
    let pc = 0x420001c4;
    let regs = XtensaRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x9c, 0x31, 0xff, 0xff];

    let res = xtensa_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x420001c6);
    assert_eq!(res[1], 0x420001db);
}

#[test]
fn test_branching_call0() {
    let pc = 0x4200048d;
    let regs = XtensaRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x05, 0x03, 0x3c, 0xff];

    let res = xtensa_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x42000490); // this isn't really a possible target
    assert_eq!(res[1], 0x4203c4c0);
}

#[test]
fn test_branching_call4() {
    let pc = 0x42000961;
    let regs = XtensaRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x95, 0x00, 0x42, 0xff];

    let res = xtensa_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x42000964); // this isn't really a possible target
    assert_eq!(res[1], 0x4204296c);
}

#[test]
fn test_branching_callx0() {
    let pc = 0x42000039;
    let regs = XtensaRegisters {
        pc,
        a4: 0x42424242,
        ..Default::default()
    };
    let isn = [0xc0, 0x04, 0x00, 0xff];

    let res = xtensa_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x4200003c); // this isn't really a possible target
    assert_eq!(res[1], 0x42424242);
}

#[test]
fn test_branching_call4_negative() {
    let pc = 0x403790e7;
    let regs = XtensaRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x15, 0x55, 0xff, 0xff];

    let res = xtensa_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x403790ea); // this isn't really a possible target
    assert_eq!(res[1], 0x40378638);
}

#[test]
fn test_branching_jump() {
    let pc = 0x42004179;
    let regs = XtensaRegisters {
        pc,
        ..Default::default()
    };
    let isn = [0x46, 0xeb, 0xff, 0xff];

    let res = xtensa_insn_estimator(isn, pc, regs);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0], 0x4200417c); // this isn't really a possible target
    assert_eq!(res[1], 0x4200412a);
}
