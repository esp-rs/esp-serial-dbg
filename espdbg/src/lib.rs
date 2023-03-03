use std::{
    fmt::Error,
    str::FromStr,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

use serialport::SerialPort;

const MESSAGE_START: u8 = 0x02;
const MESSAGE_END: u8 = 0x03;

const READ_MEM_CMD: u8 = 0x00;
const SET_BREAKPOINT_CMD: u8 = 0x01;
const CLEAR_BREAKPOINT_CMD: u8 = 0x02;
const WRITE_MEM_CMD: u8 = 0x03;
const RESUME_CMD: u8 = 0xff;
const BREAK_CMD: u8 = 0xfe;
const HELLO_CMD: u8 = 0x04;

const READ_MEM_RESPONSE: u8 = 0x00;
const HIT_BREAKPOINT_RESPONSE: u8 = 0x01;
const ACK_RESPONSE: u8 = 0x02;
const HELLO_RESPONSE: u8 = 0x03;

pub const CHIP_ESP32: u8 = 0;
pub const CHIP_ESP32S2: u8 = 1;
pub const CHIP_ESP32S3: u8 = 2;
pub const CHIP_ESP32C3: u8 = 3;
pub const CHIP_ESP32C2: u8 = 4;
pub const CHIP_ESP32C6: u8 = 5;

pub const SUPPORTED_PROTOCOL_VERSION: u32 = 0;

#[derive(Debug, Clone)]
pub enum DeviceMessage {
    MemoryDump(Vec<u8>),
    HitBreakpoint(Registers),
    Ack,
    Hello { chip: u8, protocol_version: u32 },
}

// TODO hide those pub fields!
pub struct SerialDebugConnection {
    pub chip: Chip,
    port: Arc<Mutex<Box<dyn SerialPort>>>,
    pub muted: Arc<Mutex<bool>>,
    pub wait_response: Arc<Mutex<bool>>,
    sender: Arc<Mutex<Sender<Vec<u8>>>>,
    receiver: Arc<Mutex<Receiver<Vec<u8>>>>,
    messages: Arc<Mutex<Vec<DeviceMessage>>>,
    shutdown: Arc<Mutex<bool>>,
}

impl SerialDebugConnection {
    pub fn new(chip: Chip, port: Box<dyn SerialPort>) -> SerialDebugConnection {
        let (sender, receiver) = channel::<Vec<u8>>();

        SerialDebugConnection {
            chip,
            port: Arc::new(Mutex::new(port)),
            muted: Arc::new(Mutex::new(false)),
            wait_response: Arc::new(Mutex::new(false)),
            sender: Arc::new(Mutex::new(sender)),
            receiver: Arc::new(Mutex::new(receiver)),
            messages: Arc::new(Mutex::new(Vec::new())),
            shutdown: Arc::new(Mutex::new(false)),
        }
    }

    pub fn shutdown(&self) {
        *self.shutdown.lock().unwrap() = true;
    }

    pub fn start(&self) {
        let mut rcv_packet = false;
        let mut packet: Vec<u8> = Vec::new();
        let mut pkt_len = 0;

        loop {
            if *self.shutdown.lock().unwrap() == true {
                break;
            }

            if self.port.lock().unwrap().bytes_to_read().unwrap() > 0 {
                let mut buf = [0u8];
                if let Ok(n) = self.port.lock().unwrap().read(&mut buf) {
                    if n == 1 {
                        let byte = buf[0];

                        if !rcv_packet {
                            if byte == MESSAGE_START {
                                packet.push(byte);
                                rcv_packet = true;
                            } else {
                                if *self.muted.lock().unwrap() == false {
                                    print!("{}", byte as char);
                                }
                            }
                        } else {
                            packet.push(byte);

                            if pkt_len != 0 && packet.len() >= pkt_len as usize {
                                self.handle_packet(packet.clone());

                                packet.clear();
                                rcv_packet = false;
                                *self.wait_response.lock().unwrap() = false;
                            } else {
                                if packet.len() >= 6 {
                                    let mut bytes = [0u8; 4];
                                    bytes.copy_from_slice(&packet[2..][..4]);
                                    pkt_len = u32::from_le_bytes(bytes) + 2;
                                }
                            }
                        }
                    }
                }
            }

            if let Ok(c) = self.receiver.lock().unwrap().try_recv() {
                self.port.lock().unwrap().write_all(&c[..]).unwrap();
                self.port.lock().unwrap().flush().unwrap();
            }
        }
    }

    pub fn pending_message(&self) -> Option<DeviceMessage> {
        self.messages.lock().unwrap().pop()
    }

    pub fn hello(&self, timeout: std::time::Duration) -> Result<(u8, u32), ()> {
        let pkt_len: u32 = 4 + 1;
        let mut cmd: Vec<u8> = Vec::new();
        cmd.push(MESSAGE_START); // cmd start
        cmd.push(HELLO_CMD); // write mem
        cmd.extend_from_slice(&pkt_len.to_le_bytes());
        cmd.push(MESSAGE_END); // end of cmd
        self.sender.lock().unwrap().send(cmd).unwrap();

        let started = std::time::SystemTime::now();
        loop {
            if let Some(msg) = self.messages.lock().unwrap().pop() {
                if let DeviceMessage::Hello {
                    chip,
                    protocol_version,
                } = msg
                {
                    return Ok((chip, protocol_version));
                }
            }

            if std::time::SystemTime::now() > started + timeout {
                return Err(());
            }
        }
    }

    pub fn read_memory(&self, addr: u32, len: u32) -> Vec<u8> {
        let pkt_len: u32 = 9 + 4;
        let mut cmd: Vec<u8> = Vec::new();
        cmd.push(MESSAGE_START); // cmd start
        cmd.push(READ_MEM_CMD); // read mem
        cmd.extend_from_slice(&pkt_len.to_le_bytes());
        cmd.extend_from_slice(&addr.to_le_bytes());
        cmd.extend_from_slice(&len.to_le_bytes());
        cmd.push(MESSAGE_END); // end of cmd
        self.sender.lock().unwrap().send(cmd).unwrap();

        loop {
            if let Some(DeviceMessage::MemoryDump(data)) = self.messages.lock().unwrap().pop() {
                return data;
            }
        }
    }

    pub fn write_memory(&self, addr: u32, data: Vec<u8>) {
        let pkt_len: u32 = 4 + 4 + 1 + (data.len() as u32);
        let mut cmd: Vec<u8> = Vec::new();
        cmd.push(MESSAGE_START); // cmd start
        cmd.push(WRITE_MEM_CMD); // write mem
        cmd.extend_from_slice(&pkt_len.to_le_bytes());
        cmd.extend_from_slice(&addr.to_le_bytes());
        for b in data {
            cmd.push(b);
        }
        cmd.push(MESSAGE_END); // end of cmd
        self.sender.lock().unwrap().send(cmd).unwrap();

        loop {
            if let Some(DeviceMessage::Ack) = self.messages.lock().unwrap().pop() {
                return;
            }
        }
    }

    pub fn set_breakpoint(&self, addr: u32, id: u8) {
        let pkt_len: u32 = 4 + 4 + 1 + 1;
        let mut cmd: Vec<u8> = Vec::new();
        cmd.push(MESSAGE_START); // cmd start
        cmd.push(SET_BREAKPOINT_CMD); // set bkpt
        cmd.extend_from_slice(&pkt_len.to_le_bytes());
        cmd.extend_from_slice(&addr.to_le_bytes());
        cmd.push(id);
        cmd.push(MESSAGE_END); // end of cmd

        self.sender.lock().unwrap().send(cmd).unwrap();

        loop {
            if let Some(DeviceMessage::Ack) = self.messages.lock().unwrap().pop() {
                return;
            }
        }
    }

    pub fn clear_breakpoint(&self, id: u8) {
        let pkt_len: u32 = 4 + 1 + 1;
        let mut cmd: Vec<u8> = Vec::new();
        cmd.push(MESSAGE_START); // cmd start
        cmd.push(CLEAR_BREAKPOINT_CMD); // clr bkpt
        cmd.extend_from_slice(&pkt_len.to_le_bytes());
        cmd.push(id);
        cmd.push(MESSAGE_END); // end of cmd

        self.sender.lock().unwrap().send(cmd).unwrap();

        loop {
            if let Some(DeviceMessage::Ack) = self.messages.lock().unwrap().pop() {
                return;
            }
        }
    }

    pub fn resume(&self) {
        let pkt_len: u32 = 4 + 1;
        let mut cmd: Vec<u8> = Vec::new();
        cmd.push(MESSAGE_START); // cmd start
        cmd.push(RESUME_CMD); // resume
        cmd.extend_from_slice(&pkt_len.to_le_bytes());
        cmd.push(MESSAGE_END); // end of cmd

        self.sender.lock().unwrap().send(cmd).unwrap();

        loop {
            if let Some(DeviceMessage::Ack) = self.messages.lock().unwrap().pop() {
                return;
            }
        }
    }

    pub fn break_execution(&self) {
        let pkt_len: u32 = 4 + 1;
        let mut cmd: Vec<u8> = Vec::new();
        cmd.push(MESSAGE_START); // cmd start
        cmd.push(BREAK_CMD); // resume
        cmd.extend_from_slice(&pkt_len.to_le_bytes());
        cmd.push(MESSAGE_END); // end of cmd

        self.sender.lock().unwrap().send(cmd).unwrap();
    }

    fn handle_packet(&self, packet: Vec<u8>) {
        match packet[1] {
            READ_MEM_RESPONSE => {
                let data = DeviceMessage::MemoryDump(Vec::from(&packet[6..(packet.len() - 1)]));
                self.messages.lock().unwrap().push(data);
            }
            HIT_BREAKPOINT_RESPONSE => {
                let regs = match self.chip {
                    Chip::Esp32C3 | Chip::Esp32C2 | Chip::Esp32C6 => {
                        Registers::Riscv(RiscvRegisters::from_bytes(&packet[6..(packet.len() - 1)]))
                    }
                    Chip::Esp32 | Chip::Esp32S2 | Chip::Esp32S3 => Registers::Xtensa(
                        XtensaRegisters::from_bytes(&packet[6..(packet.len() - 1)]),
                    ),
                };
                self.messages
                    .lock()
                    .unwrap()
                    .push(DeviceMessage::HitBreakpoint(regs));
            }
            ACK_RESPONSE => {
                self.messages.lock().unwrap().push(DeviceMessage::Ack);
            }
            HELLO_RESPONSE => {
                let chip = packet[6];
                let protocol_version = u32::from_le_bytes(packet[7..][..4].try_into().unwrap());
                let hello = DeviceMessage::Hello {
                    chip,
                    protocol_version,
                };
                self.messages.lock().unwrap().push(hello);
            }
            _ => panic!("received unknown packet {}", packet[1]),
        }
    }

    pub fn reset_target(&self) {
        let mut serial = self.port.lock().unwrap();
        serial.write_data_terminal_ready(false).ok();
        serial.write_request_to_send(true).ok();
        std::thread::sleep(std::time::Duration::from_millis(100));
        serial.write_request_to_send(false).ok();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Chip {
    Esp32C2,
    Esp32C3,
    Esp32C6,
    Esp32,
    Esp32S2,
    Esp32S3,
}

impl FromStr for Chip {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "esp32c2" => Ok(Chip::Esp32C2),
            "esp32-c2" => Ok(Chip::Esp32C2),
            "esp32c3" => Ok(Chip::Esp32C3),
            "esp32-c3" => Ok(Chip::Esp32C3),
            "esp32c6" => Ok(Chip::Esp32C6),
            "esp32-c6" => Ok(Chip::Esp32C6),
            "esp32" => Ok(Chip::Esp32),
            "esp32-s2" => Ok(Chip::Esp32S2),
            "esp32s2" => Ok(Chip::Esp32S2),
            "esp32-s3" => Ok(Chip::Esp32S3),
            "esp32s3" => Ok(Chip::Esp32S3),
            _ => Err(Error {}),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Registers {
    Riscv(RiscvRegisters),
    Xtensa(XtensaRegisters),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RiscvRegisters {
    pub ra: u32,
    pub t0: u32,
    pub t1: u32,
    pub t2: u32,
    pub t3: u32,
    pub t4: u32,
    pub t5: u32,
    pub t6: u32,
    pub a0: u32,
    pub a1: u32,
    pub a2: u32,
    pub a3: u32,
    pub a4: u32,
    pub a5: u32,
    pub a6: u32,
    pub a7: u32,
    pub s0: u32,
    pub s1: u32,
    pub s2: u32,
    pub s3: u32,
    pub s4: u32,
    pub s5: u32,
    pub s6: u32,
    pub s7: u32,
    pub s8: u32,
    pub s9: u32,
    pub s10: u32,
    pub s11: u32,
    pub gp: u32,
    pub tp: u32,
    pub sp: u32,
    pub pc: u32,
}

impl RiscvRegisters {
    pub fn from_bytes(data: &[u8]) -> RiscvRegisters {
        RiscvRegisters {
            ra: u32::from_le_bytes(data[0..][..4].try_into().unwrap()),
            t0: u32::from_le_bytes(data[4..][..4].try_into().unwrap()),
            t1: u32::from_le_bytes(data[8..][..4].try_into().unwrap()),
            t2: u32::from_le_bytes(data[12..][..4].try_into().unwrap()),
            t3: u32::from_le_bytes(data[16..][..4].try_into().unwrap()),
            t4: u32::from_le_bytes(data[20..][..4].try_into().unwrap()),
            t5: u32::from_le_bytes(data[24..][..4].try_into().unwrap()),
            t6: u32::from_le_bytes(data[28..][..4].try_into().unwrap()),
            a0: u32::from_le_bytes(data[32..][..4].try_into().unwrap()),
            a1: u32::from_le_bytes(data[36..][..4].try_into().unwrap()),
            a2: u32::from_le_bytes(data[40..][..4].try_into().unwrap()),
            a3: u32::from_le_bytes(data[44..][..4].try_into().unwrap()),
            a4: u32::from_le_bytes(data[48..][..4].try_into().unwrap()),
            a5: u32::from_le_bytes(data[52..][..4].try_into().unwrap()),
            a6: u32::from_le_bytes(data[56..][..4].try_into().unwrap()),
            a7: u32::from_le_bytes(data[60..][..4].try_into().unwrap()),
            s0: u32::from_le_bytes(data[64..][..4].try_into().unwrap()),
            s1: u32::from_le_bytes(data[68..][..4].try_into().unwrap()),
            s2: u32::from_le_bytes(data[72..][..4].try_into().unwrap()),
            s3: u32::from_le_bytes(data[76..][..4].try_into().unwrap()),
            s4: u32::from_le_bytes(data[80..][..4].try_into().unwrap()),
            s5: u32::from_le_bytes(data[84..][..4].try_into().unwrap()),
            s6: u32::from_le_bytes(data[88..][..4].try_into().unwrap()),
            s7: u32::from_le_bytes(data[92..][..4].try_into().unwrap()),
            s8: u32::from_le_bytes(data[96..][..4].try_into().unwrap()),
            s9: u32::from_le_bytes(data[100..][..4].try_into().unwrap()),
            s10: u32::from_le_bytes(data[104..][..4].try_into().unwrap()),
            s11: u32::from_le_bytes(data[108..][..4].try_into().unwrap()),
            gp: u32::from_le_bytes(data[112..][..4].try_into().unwrap()),
            tp: u32::from_le_bytes(data[116..][..4].try_into().unwrap()),
            sp: u32::from_le_bytes(data[120..][..4].try_into().unwrap()),
            pc: u32::from_le_bytes(data[124..][..4].try_into().unwrap()),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct XtensaRegisters {
    pub pc: u32,
    pub ps: u32,
    pub a0: u32,
    pub a1: u32,
    pub a2: u32,
    pub a3: u32,
    pub a4: u32,
    pub a5: u32,
    pub a6: u32,
    pub a7: u32,
    pub a8: u32,
    pub a9: u32,
    pub a10: u32,
    pub a11: u32,
    pub a12: u32,
    pub a13: u32,
    pub a14: u32,
    pub a15: u32,
    pub sar: u32,
    pub exccause: u32,
    pub excvaddr: u32,
    pub lbeg: u32,
    pub lend: u32,
    pub lcount: u32,
    pub threadptr: u32,
    pub scompare1: u32,
    pub br: u32,
    pub acclo: u32,
    pub acchi: u32,
    pub m0: u32,
    pub m1: u32,
    pub m2: u32,
    pub m3: u32,
    pub f64r_lo: u32,
    pub f64r_hi: u32,
    pub f64s: u32,
    pub fcr: u32,
    pub fsr: u32,
    pub f0: u32,
    pub f1: u32,
    pub f2: u32,
    pub f3: u32,
    pub f4: u32,
    pub f5: u32,
    pub f6: u32,
    pub f7: u32,
    pub f8: u32,
    pub f9: u32,
    pub f10: u32,
    pub f11: u32,
    pub f12: u32,
    pub f13: u32,
    pub f14: u32,
    pub f15: u32,
}

impl XtensaRegisters {
    pub fn from_bytes(data: &[u8]) -> XtensaRegisters {
        XtensaRegisters {
            pc: u32::from_le_bytes(data[0..][..4].try_into().unwrap()),
            ps: u32::from_le_bytes(data[4..][..4].try_into().unwrap()),
            a0: u32::from_le_bytes(data[8..][..4].try_into().unwrap()),
            a1: u32::from_le_bytes(data[12..][..4].try_into().unwrap()),
            a2: u32::from_le_bytes(data[16..][..4].try_into().unwrap()),
            a3: u32::from_le_bytes(data[20..][..4].try_into().unwrap()),
            a4: u32::from_le_bytes(data[24..][..4].try_into().unwrap()),
            a5: u32::from_le_bytes(data[28..][..4].try_into().unwrap()),
            a6: u32::from_le_bytes(data[32..][..4].try_into().unwrap()),
            a7: u32::from_le_bytes(data[36..][..4].try_into().unwrap()),
            a8: u32::from_le_bytes(data[40..][..4].try_into().unwrap()),
            a9: u32::from_le_bytes(data[44..][..4].try_into().unwrap()),
            a10: u32::from_le_bytes(data[48..][..4].try_into().unwrap()),
            a11: u32::from_le_bytes(data[52..][..4].try_into().unwrap()),
            a12: u32::from_le_bytes(data[56..][..4].try_into().unwrap()),
            a13: u32::from_le_bytes(data[60..][..4].try_into().unwrap()),
            a14: u32::from_le_bytes(data[64..][..4].try_into().unwrap()),
            a15: u32::from_le_bytes(data[68..][..4].try_into().unwrap()),
            sar: u32::from_le_bytes(data[72..][..4].try_into().unwrap()),
            exccause: u32::from_le_bytes(data[76..][..4].try_into().unwrap()),
            excvaddr: u32::from_le_bytes(data[80..][..4].try_into().unwrap()),
            lbeg: u32::from_le_bytes(data[84..][..4].try_into().unwrap()),
            lend: u32::from_le_bytes(data[88..][..4].try_into().unwrap()),
            lcount: u32::from_le_bytes(data[92..][..4].try_into().unwrap()),
            threadptr: u32::from_le_bytes(data[96..][..4].try_into().unwrap()),
            scompare1: u32::from_le_bytes(data[100..][..4].try_into().unwrap()),
            br: u32::from_le_bytes(data[104..][..4].try_into().unwrap()),
            acclo: u32::from_le_bytes(data[108..][..4].try_into().unwrap()),
            acchi: u32::from_le_bytes(data[112..][..4].try_into().unwrap()),
            m0: u32::from_le_bytes(data[116..][..4].try_into().unwrap()),
            m1: u32::from_le_bytes(data[120..][..4].try_into().unwrap()),
            m2: u32::from_le_bytes(data[124..][..4].try_into().unwrap()),
            m3: u32::from_le_bytes(data[128..][..4].try_into().unwrap()),
            f64r_lo: u32::from_le_bytes(data[132..][..4].try_into().unwrap()),
            f64r_hi: u32::from_le_bytes(data[136..][..4].try_into().unwrap()),
            f64s: u32::from_le_bytes(data[140..][..4].try_into().unwrap()),
            fcr: u32::from_le_bytes(data[144..][..4].try_into().unwrap()),
            fsr: u32::from_le_bytes(data[148..][..4].try_into().unwrap()),
            f0: u32::from_le_bytes(data[152..][..4].try_into().unwrap()),
            f1: u32::from_le_bytes(data[156..][..4].try_into().unwrap()),
            f2: u32::from_le_bytes(data[160..][..4].try_into().unwrap()),
            f3: u32::from_le_bytes(data[164..][..4].try_into().unwrap()),
            f4: u32::from_le_bytes(data[168..][..4].try_into().unwrap()),
            f5: u32::from_le_bytes(data[172..][..4].try_into().unwrap()),
            f6: u32::from_le_bytes(data[176..][..4].try_into().unwrap()),
            f7: u32::from_le_bytes(data[180..][..4].try_into().unwrap()),
            f8: u32::from_le_bytes(data[184..][..4].try_into().unwrap()),
            f9: u32::from_le_bytes(data[188..][..4].try_into().unwrap()),
            f10: u32::from_le_bytes(data[192..][..4].try_into().unwrap()),
            f11: u32::from_le_bytes(data[196..][..4].try_into().unwrap()),
            f12: u32::from_le_bytes(data[200..][..4].try_into().unwrap()),
            f13: u32::from_le_bytes(data[204..][..4].try_into().unwrap()),
            f14: u32::from_le_bytes(data[208..][..4].try_into().unwrap()),
            f15: u32::from_le_bytes(data[212..][..4].try_into().unwrap()),
        }
    }
}
