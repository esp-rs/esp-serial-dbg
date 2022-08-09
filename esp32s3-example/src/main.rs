#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

use core::cell::RefCell;

use esp32s3_hal::{
    clock::ClockControl,
    interrupt::{self, Priority},
    pac::{self, Peripherals, TIMG0},
    prelude::*,
    timer::{Timer, Timer0, TimerGroup},
    Delay, Rtc, Serial,
};
use esp_backtrace as _;
use esp_println::println;
use xtensa_lx::mutex::Mutex;
use xtensa_lx::mutex::SpinLockMutex;
use xtensa_lx_rt::entry;

static mut TIMER00: SpinLockMutex<RefCell<Option<Timer<Timer0<TIMG0>>>>> =
    SpinLockMutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    let mut timer00 = timer_group0.timer0;
    interrupt::enable(pac::Interrupt::TG0_T0_LEVEL, Priority::Priority2).unwrap();
    timer00.start(500u64.millis());
    timer00.listen();

    unsafe {
        (&TIMER00).lock(|data| (*data).replace(Some(timer00)));
    }

    esp_serial_dbg::init(Serial::new(peripherals.UART0));

    let mut delay = Delay::new(&clocks);

    let mut i = 0;
    println!("ok");

    loop {
        some_ram_function(i);
        i = i.wrapping_add(1);
        delay.delay_ms(1500u32);
        some_function(i);
        i = i.wrapping_add(1);
        delay.delay_ms(1500u32);
    }
}

fn some_function(param: u32) {
    println!("hello {}", param);
}

#[ram]
fn some_ram_function(param: u32) {
    // also SW breakpoints work
    let mut x = 0;
    println!("hello from ram function! {}", param);
    x += 1;
    println!("hello from ram function! x={}", x);
}

#[interrupt]
fn TG0_T0_LEVEL() {
    unsafe {
        (&TIMER00).lock(|data| {
            let mut timer = data.borrow_mut();
            let timer = timer.as_mut().unwrap();

            in_timer();

            if timer.is_interrupt_set() {
                timer.clear_interrupt();
                timer.start(500u64.millis());
            }
        });
    }
}

fn in_timer() {
    println!("timer");
}
