#![no_std]
#![no_main]

use core::cell::RefCell;

use esp32c3_hal::{
    clock::ClockControl,
    interrupt,
    pac::{self, Peripherals, TIMG0},
    prelude::*,
    timer::{Timer, Timer0, TimerGroup},
    Delay, Rtc, Serial,
};
use esp_backtrace as _;
use esp_println::println;
use riscv::interrupt::Mutex;
use riscv_rt::entry;

static mut TIMER0: Mutex<RefCell<Option<Timer<Timer0<TIMG0>>>>> = Mutex::new(RefCell::new(None));

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

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    esp_serial_dbg::init(Serial::new(peripherals.UART0));

    let mut delay = Delay::new(&clocks);

    let mut timer0 = timer_group0.timer0;
    interrupt::enable(pac::Interrupt::TG0_T0_LEVEL, interrupt::Priority::Priority1).unwrap();
    timer0.start(2500u64.millis());
    timer0.listen();

    riscv::interrupt::free(|_cs| unsafe {
        TIMER0.get_mut().replace(Some(timer0));
    });

    let mut i = 0;
    println!("ok");

    // breakpoints here work fine
    loop {
        some_function(i);
        i = i.wrapping_add(1);
        delay.delay_ms(1500u32);
        some_function(i);
        i = i.wrapping_add(1);
        delay.delay_ms(1500u32);
        riscv::interrupt::free(|_| {
            // also here breakpoints work
            some_ram_function(i);
        });
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

fn called_by_interrupt_handler() {
    esp_println::println!("Interrupt 1, again!");
}

#[interrupt]
fn TG0_T0_LEVEL() {
    // unfortunately no breakpoints inside interrupt handlers
    esp_println::println!("Interrupt 1");
    called_by_interrupt_handler();

    riscv::interrupt::free(|cs| unsafe {
        let mut timer0 = TIMER0.borrow(*cs).borrow_mut();
        let timer0 = timer0.as_mut().unwrap();

        timer0.clear_interrupt();
        timer0.start(500u64.millis());
    });
}
