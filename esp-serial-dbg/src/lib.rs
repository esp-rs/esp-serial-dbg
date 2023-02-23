#![no_std]
#![cfg_attr(target_arch = "xtensa", feature(asm_experimental_arch))]

use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::serial::Read;
use embedded_hal::serial::Write;
#[cfg(feature = "esp32")]
pub use esp32_hal as hal;
#[cfg(feature = "esp32c3")]
pub use esp32c3_hal as hal;
#[cfg(feature = "esp32c2")]
pub use esp32c2_hal as hal;
#[cfg(feature = "esp32s2")]
pub use esp32s2_hal as hal;
#[cfg(feature = "esp32s3")]
pub use esp32s3_hal as hal;
#[allow(unused)]
use hal::interrupt;
use hal::macros::interrupt;
use hal::prelude::nb;
use hal::{uart::config::AtCmdConfig, Uart};

const MAX_COMMAND_BUFFER: usize = 256;

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

pub const PROTOCOL_VERSION: u32 = 0;

#[cfg_attr(target_arch = "riscv32", path = "riscv.rs")]
#[cfg_attr(target_arch = "xtensa", path = "xtensa.rs")]
pub mod arch;

type ExceptionContext = crate::hal::trapframe::TrapFrame;

static SERIAL: Mutex<RefCell<Option<Uart<hal::peripherals::UART0>>>> = Mutex::new(RefCell::new(None));

pub fn store_serial(serial: Uart<'static, hal::peripherals::UART0>) {
    critical_section::with(|cs| {
        SERIAL.borrow(cs).replace(Some(serial));
    });
}

pub fn with_serial<F, R>(f: F) -> R
where
    F: FnOnce(&mut Uart<hal::peripherals::UART0>) -> R,
{
    critical_section::with(|cs| {
        let mut serial = SERIAL.borrow(cs).borrow_mut();
        let serial = serial.as_mut().unwrap();

        f(serial)
    })
}

/// Currently only UART0 supported
pub fn init(mut serial: Uart<'static, hal::peripherals::UART0>) {
    arch::init();

    serial.set_at_cmd(AtCmdConfig::new(
        Some(0),
        Some(0),
        Some(8),
        MESSAGE_START,
        Some(1),
    ));

    serial.listen_at_cmd();

    hal::interrupt::enable(
        hal::peripherals::Interrupt::UART0,
        #[cfg(target_arch = "riscv32")]
        hal::interrupt::Priority::Priority14,
        #[cfg(target_arch = "xtensa")]
        hal::interrupt::Priority::Priority3,
    )
    .unwrap();

    store_serial(serial);

    #[cfg(target_arch = "riscv32")]
    unsafe {
        hal::riscv::interrupt::enable();
    }
}

#[interrupt]
fn UART0(trap_frame: &mut ExceptionContext) {
    critical_section::with(|_cs| {
        #[cfg(target_arch = "riscv32")]
        let mepc = hal::riscv::register::mepc::read();
        #[cfg(target_arch = "xtensa")]
        let mepc = trap_frame.PC as usize;
        with_serial(|serial| {
            let mut buffer = [0u8; MAX_COMMAND_BUFFER];
            let len = read_command(serial, &mut buffer);
            handle_cmd(&buffer, len, serial, trap_frame, mepc, false);

            serial.reset_at_cmd_interrupt();
        });
    });
}

fn serial_com_halted(
    serial: &mut Uart<hal::peripherals::UART0>,
    trap_frame: &mut ExceptionContext,
    mepc: usize,
) {
    let mut buffer = [0u8; MAX_COMMAND_BUFFER];
    loop {
        let len = read_command(serial, &mut buffer);
        serial.reset_at_cmd_interrupt();

        if buffer[1] == RESUME_CMD {
            send_ack(serial);
            return;
        } else {
            handle_cmd(&buffer, len, serial, trap_frame, mepc, true);
        }
    }
}

fn read_command(
    serial: &mut Uart<hal::peripherals::UART0>,
    buffer: &mut [u8; MAX_COMMAND_BUFFER],
) -> usize {
    let mut cnt = 0;
    let mut len = 0;
    loop {
        while let nb::Result::Ok(c) = serial.read() {
            buffer[cnt] = c;
            cnt += 1;
        }

        if cnt >= 6 && len == 0 {
            len = u32::from_le_bytes(buffer[2..][..4].try_into().unwrap()) as usize;
        }

        if len != 0 && cnt >= len + 2 {
            break;
        }
    }
    cnt
}

fn handle_cmd(
    buffer: &[u8; 256],
    cnt: usize,
    serial: &mut Uart<hal::peripherals::UART0>,
    trap_frame: &mut ExceptionContext,
    mepc: usize,
    from_halted_state: bool,
) {
    match buffer[1] {
        READ_MEM_CMD => readmem_cmd(buffer, cnt, serial),
        SET_BREAKPOINT_CMD => set_breakpoint(buffer, cnt, serial),
        CLEAR_BREAKPOINT_CMD => clear_breakpoint(buffer, cnt, serial),
        BREAK_CMD => handle_break(serial, trap_frame, mepc, from_halted_state),
        WRITE_MEM_CMD => writemem_cmd(buffer, cnt, serial),
        HELLO_CMD => handle_hello(buffer, cnt, serial),
        _ => {}
    }
}

fn handle_break(
    serial: &mut Uart<hal::peripherals::UART0>,
    context: &mut ExceptionContext,
    mepc: usize,
    from_halted_state: bool,
) {
    // breakpoint
    critical_section::with(|_cs| {
        let regs_raw = arch::serialze_registers(context, mepc);

        crate::write_response(
            serial,
            crate::HIT_BREAKPOINT_RESPONSE,
            regs_raw.len(),
            &mut regs_raw.into_iter(),
        );

        if !from_halted_state {
            // process commands while halted
            crate::serial_com_halted(serial, context, mepc);
        }
    });
}

fn send_ack(serial: &mut Uart<hal::peripherals::UART0>) {
    write_response(serial, ACK_RESPONSE, 0, &mut [].into_iter());
}

fn readmem_cmd(buffer: &[u8; 256], _cnt: usize, serial: &mut Uart<hal::peripherals::UART0>) {
    let addr = u32::from_le_bytes(buffer[6..][..4].try_into().unwrap());
    let len = u32::from_le_bytes(buffer[10..][..4].try_into().unwrap());
    let len_aligned = if len % 4 != 0 {
        len + (4 - (len % 4))
    } else {
        len
    };

    let mut mem_reader = MemReader::new(addr, len_aligned);
    write_response(
        serial,
        READ_MEM_RESPONSE,
        len_aligned as usize,
        &mut mem_reader,
    );
}

fn writemem_cmd(buffer: &[u8; 256], cnt: usize, serial: &mut Uart<hal::peripherals::UART0>) {
    let addr = u32::from_le_bytes(buffer[6..][..4].try_into().unwrap());

    let ptr = addr as *mut u32;
    unsafe {
        for i in (10..(cnt - 1)).step_by(4) {
            let src_ptr = &buffer[i] as *const _ as *const u32;
            let word = src_ptr.read_unaligned();
            ptr.offset(((i - 10) / 4) as isize).write_volatile(word);
        }
    }

    send_ack(serial);
}

fn set_breakpoint(buffer: &[u8; 256], _cnt: usize, serial: &mut Uart<hal::peripherals::UART0>) {
    let addr = u32::from_le_bytes(buffer[6..][..4].try_into().unwrap());
    let id = buffer[10];
    arch::set_breakpoint(id, addr);

    send_ack(serial);
}

fn clear_breakpoint(buffer: &[u8; 256], _cnt: usize, serial: &mut Uart<hal::peripherals::UART0>) {
    let id = buffer[6];
    arch::clear_breakpoint(id);

    send_ack(serial);
}

fn handle_hello(_buffer: &[u8; 256], _cnt: usize, serial: &mut Uart<hal::peripherals::UART0>) {
    #[cfg(feature = "esp32")]
    let chip = CHIP_ESP32;

    #[cfg(feature = "esp32s2")]
    let chip = CHIP_ESP32S2;

    #[cfg(feature = "esp32s3")]
    let chip = CHIP_ESP32S3;

    #[cfg(feature = "esp32c3")]
    let chip = CHIP_ESP32C3;

    #[cfg(feature = "esp32c2")]
    let chip = CHIP_ESP32C2;

    let mut payload = [chip, 0, 0, 0, 0];

    payload[1..].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
    write_response(
        serial,
        HELLO_RESPONSE,
        payload.len(),
        &mut payload.into_iter(),
    );
}

fn write_response(
    serial: &mut Uart<hal::peripherals::UART0>,
    id: u8,
    payload_len: usize,
    payload: &mut dyn Iterator<Item = u8>,
) {
    nb::block!(serial.write(MESSAGE_START)).unwrap(); // start of response
    nb::block!(serial.write(id)).unwrap(); // read mem response

    serial
        .write_bytes(&(payload_len + 4 + 1).to_le_bytes())
        .unwrap();
    loop {
        let b = payload.next();
        match b {
            Some(b) => nb::block!(serial.write(b)).unwrap(),
            None => break,
        }
    }

    nb::block!(serial.write(MESSAGE_END)).unwrap(); // end of response
    nb::block!(serial.flush()).unwrap();
}

struct MemReader {
    ptr: *const u32,
    size: usize,
    index: usize,
}

impl MemReader {
    fn new(address: u32, len: u32) -> MemReader {
        MemReader {
            ptr: address as *const u32,
            size: len as usize,
            index: 0,
        }
    }
}

impl Iterator for MemReader {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.size {
            None
        } else {
            let word = unsafe { self.ptr.offset(self.index as isize / 4).read_volatile() };
            let res = Some(word.to_le_bytes()[self.index % 4]);
            self.index += 1;
            res
        }
    }
}
